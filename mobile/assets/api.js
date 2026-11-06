import {profileDetails} from "./details.js";

// eCampus embeds JSON.parse("…") in an otherwise JSON object literal.
// Read quoted tokens as units so braces and function names in course titles stay intact.
export function viewModel(script) {
    const match = /\b(?:var|let|const)\s+viewModel\s*=\s*/.exec(script);
    if (!match) throw new Error("На странице отсутствует viewModel.");
    let pos = match.index + match[0].length;
    let json = "", depth = 0;
    const quoted = () => {
        const start = pos++;
        while (pos < script.length) {
            const c = script[pos++];
            if (c === "\\") pos++;
            else if (c === '"') return script.slice(start, pos);
        }
        throw new Error("Строка в ответе eCampus повреждена.");
    };
    if (script[pos] !== "{") throw new Error("Неизвестный формат viewModel.");
    while (pos < script.length) {
        if (script[pos] === '"') { json += quoted(); continue; }
        if (script.startsWith("JSON.parse", pos)) {
            const opening = /^JSON\.parse\s*\(\s*/.exec(script.slice(pos));
            if (!opening) throw new Error("Неизвестный формат данных eCampus.");
            pos += opening[0].length;
            if (script[pos] !== '"') throw new Error("Ожидалась строка JSON.");
            json += JSON.stringify(JSON.parse(JSON.parse(quoted())));
            const closing = /^\s*\)/.exec(script.slice(pos));
            if (!closing) throw new Error("Ответ eCampus повреждён.");
            pos += closing[0].length;
            continue;
        }
        const c = script[pos++];
        if (c === "{") depth++;
        json += c;
        if (c === "}" && --depth === 0) return JSON.parse(json);
    }
    throw new Error("Получен неполный ответ eCampus.");
}

const base = "https://ecampus.ncfu.ru";
function localPath(path) {
    const url = new URL(path, base);
    if (url.origin !== base || url.username || url.password) throw new Error("Неизвестный адрес eCampus.");
    return url;
}
async function request(path, form, signal) {
    const body = form == null ? undefined : new URLSearchParams(Object.entries(form).filter(([,v]) => v != null));
    const response = await fetch(localPath(path), {method: body ? "POST" : "GET", body,
        credentials: "same-origin", cache: "no-store", signal});
    if ([401,403].includes(response.status) && !path.toLowerCase().includes("login")) throw new Error("expired");
    if (!response.ok) throw new Error("Ошибка eCampus (HTTP " + response.status + ").");
    return response;
}
function parseHtml(html) {
    const doc = document.createElement("template");
    doc.innerHTML = html;
    return doc.content;
}
function model(doc, requiredField = "") {
    const script = [...doc.querySelectorAll("script")].find(s => s.textContent.includes(requiredField) && /\b(?:var|let|const)\s+viewModel\s*=/.test(s.textContent));
    if (!script) throw new Error("Страница eCampus изменилась. Не удалось найти данные.");
    return viewModel(script.textContent);
}
function checkArray(value) {
    if (!Array.isArray(value)) throw new Error("Неизвестный формат списка eCampus.");
    return value;
}
async function dataUrl(blob) {
    return new Promise((resolve,reject) => {
        const reader = new FileReader();
        reader.onload = () => resolve(reader.result);
        reader.onerror = () => reject(new Error("Не удалось прочитать капчу."));
        reader.readAsDataURL(blob);
    });
}
export async function execute(job, signal) {
    if (location.origin !== base) throw new Error("Вход в eCampus доступен в Android-приложении. Для просмотра выберите деморежим.");
    let response = await request(job.path, job.form, signal);
    if (job.format === "challenge") {
        const doc = parseHtml(await response.text());
        const token = doc.querySelector('input[name="__RequestVerificationToken"]')?.value;
        const src = doc.querySelector('img[alt="captcha" i]')?.getAttribute("src");
        if (!token || !src) throw new Error("На странице входа не удалось найти капчу.");
        const image = await request(localPath(src).href, null, signal);
        return {token,image:await dataUrl(await image.blob())};
    }
    if (job.format === "login") response = await request("/schedule/my/student", null, signal);
    const text = await response.text();
    const loginForm = job.format !== "details" && /name\s*=\s*["']__RequestVerificationToken["']/i.test(text) && /name\s*=\s*["']Password["']/i.test(text);
    if (new URL(response.url).pathname.toLowerCase().includes("/account/login") || loginForm) {
        throw new Error(job.format === "login" ? "Проверьте логин, пароль и ответ с картинки." : "expired");
    }
    if (job.format === "courses") return checkArray(JSON.parse(text).courses);
    if (job.format === "array") return checkArray(JSON.parse(text));
    const doc = parseHtml(text);
    if (job.format === "details") {
        let vm;
        try { vm = model(doc, "EducationDetails"); } catch { /* Fall back to the signed-in header. */ }
        return profileDetails(doc, vm);
    }
    const vm = model(doc);
    if (job.format === "login" || job.format === "profile") {
        if (!vm.Model || !vm.Model.Id) throw new Error("Не удалось определить ID студента.");
        return vm.Model;
    }
    if (job.format === "institutes") return checkArray(vm.Institutes);
    checkArray(vm.specialities);
    return vm;
}
