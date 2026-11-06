// /details includes one EducationDetails entry per programme. Keep only the
// fields shown on the home screen; the schedule ID comes from a different page.
export function profileDetails(doc, viewModel) {
    const clean = text => typeof text === "string" ? text.trim().replace(/\s+/g, " ") : "";
    const entries = Array.isArray(viewModel?.EducationDetails) ? viewModel.EducationDetails : [];
    const programme = entries.find(entry => entry?.isActivity === true)
        ?? entries.find(entry => entry && entry.isActivity == null);
    const fullName = clean(programme?.FullName) || entries.map(entry => clean(entry?.FullName)).find(Boolean);
    const displayName = clean(doc.querySelector(".login .username")?.textContent);
    const name = clean(programme?.SpecialtyName);
    const code = clean(programme?.Specialty);
    const course = Number(programme?.Course);

    const result = {};
    if (fullName) result.FullName = fullName;
    if (displayName) result.DisplayName = displayName;
    if (name || code) result.SpecialtyName = name && code ? name + " (" + code + ")" : name || code;
    if (Number.isInteger(course) && course > 0 && course <= 255) result.Course = course;
    if (!Object.keys(result).length) throw new Error("На странице /details не удалось прочитать данные профиля.");
    return result;
}
