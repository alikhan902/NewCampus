import {execute} from "./api.js";

const root = document.querySelector("#app");
const utf8 = new TextEncoder();
const decoder = new TextDecoder();
let wasm, generation = 0, signed = false, lastHtml = "", clearingCookies = false;
const running = new Set(), queue = [];

function dispatch(event) {
    const bytes = utf8.encode(JSON.stringify(event));
    const ptr = wasm.input_buffer(bytes.length);
    new Uint8Array(wasm.memory.buffer, ptr, bytes.length).set(bytes);
    const len = wasm.dispatch(ptr, bytes.length);
    const result = JSON.parse(decoder.decode(new Uint8Array(wasm.memory.buffer, wasm.output_buffer(), len)));
    const form = document.forms["login-form"];
    const fields = form ? Object.fromEntries(new FormData(form)) : {};
    const showPassword = form?.elements.password.type === "text";
    const focus = document.activeElement?.name;
    const scroll = root.querySelector(".scroll")?.scrollTop || 0;
    const keepScroll = Boolean(event.result);
    if (lastHtml !== result.html) {
        root.innerHTML = result.html;
        lastHtml = result.html;
        const next = document.forms["login-form"];
        if (next) {
            for (const [key,v] of Object.entries(fields)) if (next.elements[key] && !(key === "code" && event.result === "challenge")) next.elements[key].value = v;
            if (showPassword) next.elements.password.type = "text";
            if (focus && next.elements[focus] && !event.action) next.elements[focus].focus();
        }
        if (keepScroll && root.querySelector(".scroll")) root.querySelector(".scroll").scrollTop = scroll;
    }
    signed = result.signed;
    globalThis.Campus?.theme(signed);
    if (signed && !result.demo) localStorage.setItem("session", "1");
    for (const job of result.effects) {
        if (job.key === "reset") {
            invalidate();
            localStorage.removeItem("session");
            if (globalThis.Campus) {
                clearingCookies = true;
                globalThis.Campus.clearSession();
            }
        } else if (job.key === "close") globalThis.Campus?.close();
        else queue.push({job, generation});
    }
    drain();
}
function invalidate() {
    generation++;
    queue.length = 0;
    for (const controller of running) controller.abort();
}
globalThis.campusCookiesCleared = () => { clearingCookies = false; drain(); };
function drain() {
    if (clearingCookies) return;
    while (running.size < 3 && queue.length) {
        const item = queue.shift();
        const controller = new AbortController();
        running.add(controller);
        const timer = setTimeout(() => controller.abort(), 22000);
        execute(item.job, controller.signal).then(
            data => {if (item.generation === generation) dispatch({result:item.job.key,data});},
            err => {
                let msg = err.message;
                if (err.name === "AbortError" || err instanceof TypeError) msg = "Не удалось связаться с eCampus. Проверьте интернет и попробуйте ещё раз.";
                if (err instanceof SyntaxError) msg = "eCampus вернул данные в неизвестном формате.";
                if (item.generation === generation) dispatch({result:item.job.key,error:msg});
            }
        ).finally(() => { clearTimeout(timer);running.delete(controller);drain(); });
    }
}
root.addEventListener("submit", e => {
    e.preventDefault();
    if (e.target.id === "login-form") dispatch({action:"login",fields:Object.fromEntries(new FormData(e.target))});
});
root.addEventListener("click", e => {
    const eye = e.target.closest("#eye");
    if (eye) {
        const password = eye.form.elements.password;
        password.type = password.type === "password" ? "text" : "password";
        eye.setAttribute("aria-label",password.type === "password" ? "Показать пароль" : "Скрыть пароль");
        return;
    }
    const action = e.target.closest("[data-act]")?.dataset.act;
    if (!action) return;
    e.preventDefault();
    if (action === "demo") invalidate();
    dispatch({action});
});
globalThis.campusBack = () => {if (!signed) globalThis.Campus?.close(); else dispatch({action:"back"});};

async function start() {
    const binary = await (await fetch("ecampus_mobile.wasm")).arrayBuffer();
    wasm = (await WebAssembly.instantiate(binary, {})).instance.exports;
    const date = new Date();
    const today = [date.getFullYear(),String(date.getMonth()+1).padStart(2,"0"),String(date.getDate()).padStart(2,"0")].join("-");
    dispatch({today,action:localStorage.getItem("session") === "1" ? "resume" : ""});
}
start().catch(() => { root.textContent = "Не удалось открыть eCampus. Обновите Android System WebView и запустите приложение снова."; });
