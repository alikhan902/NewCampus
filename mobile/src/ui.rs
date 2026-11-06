use crate::{
    app::{App, Page},
    data::*,
};
use serde_json::Value;
use time::{Date, Duration};

pub const SERVICES: [(&str, &str, &str, &str); 11] = [
    (
        "Оценки",
        "Все оценки семестра и их распределение",
        "pie",
        "blue",
    ),
    (
        "Посещаемость",
        "Пропуски занятий: \"Н\"ки и \"У\"шки",
        "attendance",
        "red",
    ),
    (
        "Уведомления",
        "Уведомления о выставленных КТ, финансовых задолженностях и т.д.",
        "bell",
        "orange",
    ),
    (
        "Рейтинг",
        "Список ваших одногруппников и их рейтинг",
        "rank",
        "orange",
    ),
    (
        "Все преподаватели",
        "Информация о всех преподавателях СКФУ",
        "teacher",
        "gray",
    ),
    (
        "Мои преподаватели",
        "Прочитайте характеристику учителей и отзывы студентов.",
        "teacher",
        "red",
    ),
    (
        "Карта корпусов",
        "Геолокация корпусов, общежитий, мед. пункта и т.д.",
        "map",
        "clay",
    ),
    (
        "Зачётная книжка",
        "Электронная зачетная книжка",
        "calendar",
        "gray",
    ),
    (
        "Wi-Fi",
        "Сброс пароля от университетского Wi-Fi.",
        "wifi",
        "orange",
    ),
    (
        "Оплата обучения",
        "Задолженности по договору, реквизиты и оплата онлайн",
        "card",
        "green",
    ),
    (
        "Студ. офис СКФУ",
        "Здесь Вы можете подать заявку на получение услуги студенческого офиса онлайн",
        "cap",
        "gray",
    ),
];
const TABS: [(&str, &str); 4] = [
    ("Главная", "student"),
    ("Расписание", "calendar"),
    ("Предметы", "books"),
    ("Сервисы", "grid"),
];
fn icon(name: &str) -> String {
    format!(r##"<svg aria-hidden="true" class="icon"><use href="icons.svg#{name}"/></svg>"##)
}
fn tool(cmd: &str, name: &str, label: &str) -> String {
    format!(
        r#"<button type="button" class="tool" data-act="{cmd}" aria-label="{label}">{}</button>"#,
        icon(name)
    )
}
fn button(cmd: &str, label: &str, active: bool) -> String {
    format!(
        r#"<button data-act="{cmd}" class="{}">{label}</button>"#,
        if active { "active" } else { "" }
    )
}
fn read(row: &Value, keys: &[&str]) -> String {
    escape(&field(row, keys))
}
fn header(title: &str, left: String, right: String) -> String {
    format!("<header>{left}<h1>{title}</h1>{right}</header>")
}
fn fallback(s: String, default: &str) -> String {
    if s.is_empty() {
        default.into()
    } else {
        s
    }
}
fn time_text(s: String) -> String {
    s.get(11..16).or_else(|| s.get(..5)).unwrap_or("—").into()
}
fn teacher(c: &Value) -> String {
    let primary = field(c, &["Teacher", "TeacherName"]);
    if !primary.is_empty() {
        return escape(&primary);
    }
    let names: std::collections::BTreeSet<_> = list(&c["LessonTypes"])
        .iter()
        .map(|l| read(l, &["Teacher", "TeacherName"]))
        .filter(|s| !s.is_empty())
        .collect();
    names.into_iter().collect::<Vec<_>>().join(", ")
}
impl App {
    pub fn html(&self) -> String {
        if !self.signed() {
            return self.login_html();
        }
        let (title, left, right, body) = match self.page {
            Page::Tab => match self.tab {
                0 => (
                    "eCampus",
                    tool("logout", "exit", "Выйти"),
                    tool("service:2", "bell", "Уведомления"),
                    self.home(),
                ),
                1 => (
                    "Расписание",
                    tool("today", "teacher", "Моё расписание на сегодня"),
                    tool("groups", "search", "Найти группу"),
                    self.schedule(),
                ),
                2 => (
                    "Предметы",
                    tool("service:0", "pie", "Оценки"),
                    tool("refresh", "refresh", "Обновить предметы"),
                    self.subjects(),
                ),
                _ => (
                    "Сервисы",
                    "<span></span>".into(),
                    tool("service:2", "bell", "Уведомления"),
                    self.services(),
                ),
            },
            _ => {
                let (title, content) = match self.page {
                    Page::Service(i) => (SERVICES[i].0, self.service(i)),
                    Page::Subject(i) => ("Предмет", self.subject(i)),
                    _ => ("Расписание групп", self.directory_view()),
                };
                (
                    title,
                    tool("back", "left", "Назад"),
                    "<span></span>".into(),
                    format!("<main class='scroll'>{content}</main>"),
                )
            }
        };
        let nav = TABS
            .iter()
            .enumerate()
            .map(|(i, (n, ic))| {
                button(
                    &format!("tab:{i}"),
                    &format!("{}<span>{n}</span>", icon(ic)),
                    self.tab == i,
                )
            })
            .collect::<String>();
        format!(
            "<div class='shell'>{}{}<nav aria-label='Разделы'>{nav}</nav></div>",
            header(title, left, right),
            body
        )
    }
    fn login_html(&self) -> String {
        let busy = !self.pending.is_empty();
        let challenge = if self.captcha.is_null() {
            String::new()
        } else {
            format!(
                r#"<div class="captcha"><img alt="Выражение с картинки" src="{}">{}</div><label class="input"><input name="code" required autocomplete="off" inputmode="numeric" aria-label="Ответ на капчу" placeholder="Ответ с картинки"></label>"#,
                read(&self.captcha, &["image"]),
                tool("challenge", "refresh", "Обновить капчу")
            )
        };
        let msg = if self.message.is_empty() {
            String::new()
        } else {
            format!(
                "<p role='alert' class='login-msg'>{}</p>",
                escape(&self.message)
            )
        };
        format!(
            r#"<main class="login"><form id="login-form">{}<h1>eCampus</h1>
<label class="input">{}<input name="login" required autocomplete="username" aria-label="Имя пользователя" placeholder="Имя пользователя" autocapitalize="none" spellcheck="false"></label>
<label class="input">{}<input name="password" required type="password" autocomplete="current-password" aria-label="Пароль" placeholder="Пароль"><button type="button" id="eye" aria-label="Показать пароль">{}</button></label>{challenge}{msg}
<button class="submit" {}>{} {}</button></form><footer><button data-act="demo">Деморежим</button><small>Created by Focus</small></footer></main>"#,
            icon("student"),
            icon("user"),
            icon("lock"),
            icon("eye"),
            if busy { "disabled" } else { "" },
            if busy {
                "Подождите…"
            } else {
                "Войти"
            },
            icon("enter")
        )
    }
    fn status(&self, key: &str) -> Option<String> {
        if self.cache.get(key).is_some_and(|v| v.is_ok()) {
            return None;
        }
        let msg = self.cache.get(key).and_then(|v| v.as_ref().err());
        Some(format!(
            "<div class='notice' role='status'>{} {}</div>",
            msg.map(|s| escape(s)).unwrap_or_else(|| "Загрузка…".into()),
            if msg.is_some() {
                button("retry", "Повторить", false)
            } else {
                String::new()
            }
        ))
    }
    fn home(&self) -> String {
        let terms = self.terms();
        let term = terms.iter().find(|t| t.current).or_else(|| terms.last());
        let (score, n, grade_status) = self.profile_stats(&terms);
        let details = self.value("details");
        let profile_field = |key: &str, alternatives: &[&str]| {
            fallback(read(details, &[key]), &read(&self.profile, alternatives))
        };
        let full_name = profile_field("FullName", &["FullName", "Fio", "StudentName"]);
        let name = fallback(
            full_name,
            &fallback(profile_field("DisplayName", &["Name"]), "Студент"),
        );
        let spec = fallback(
            profile_field(
                "SpecialtyName",
                &["SpecialtyName", "SpecialityName", "Specialty"],
            ),
            term.map(|t| escape(&t.speciality)).as_deref().unwrap_or(""),
        );
        let group = read(&self.profile, &["GroupName", "AcademicGroup", "Group"]);
        let year = field(details, &["Course"])
            .parse::<u8>()
            .ok()
            .map(|n| format!("{n} курс"))
            .or_else(|| term.map(|t| format!("{} курс", t.year)))
            .unwrap_or("—".into());
        let profile_notice = if self.cache.get("details").is_some_and(Result::is_err) {
            format!(
                "<p class='grade-status'>Не удалось загрузить данные профиля. {}</p>",
                button("retry-details", "Повторить", false)
            )
        } else {
            String::new()
        };
        let stats = [
            ("star", score, "Средняя оценка", "service:0"),
            ("cap", year, group.as_str(), "tab:2"),
            ("attendance", n, "Пропуски (\"Н\")", "service:1"),
        ]
        .into_iter()
        .map(|(ic, num, txt, cmd)| {
            button(
                cmd,
                &format!("{}<b>{num}</b><small>{txt}</small>", icon(ic)),
                false,
            )
        })
        .collect::<String>();
        let demo = if self.demo {
            "<small class='demo-label'>Демопрофиль</small>"
        } else {
            ""
        };
        format!("<main class='scroll home'><div class='avatar'></div><h2>{name}</h2><p>{spec}</p>{demo}{profile_notice}<section class='stats'>{stats}</section>{grade_status}<section class='today'><button data-act='today'><b>Расписание</b></button>{}</section></main>",self.lessons(self.today,true))
    }
    fn lessons(&self, date: Date, home: bool) -> String {
        let key = self.week_key(date, !home);
        if let Some(s) = self.status(&key) {
            return s;
        }
        let rows = list(self.value(&key))
            .iter()
            .filter(|d| field(d, &["Date"]).starts_with(&date.to_string()))
            .flat_map(|d| list(&d["Lessons"]));
        let mut html = String::new();
        for l in rows {
            let from = time_text(read(l, &["TimeBegin"]));
            let to = time_text(read(l, &["TimeEnd"]));
            let details = [
                read(l, &["LessonType"]),
                read(l, &["Teacher"]),
                read(l, &["Aud"]),
                list(&l["Groups"])
                    .iter()
                    .map(|g| read(g, &["Name"]))
                    .collect::<Vec<_>>()
                    .join(", "),
            ];
            html.push_str(&format!("<article class='lesson'><small>{from} – {to}</small><h2>{}</h2><p>{}</p></article>",read(l,&["Discipline"]),details.into_iter().filter(|x|!x.is_empty()).collect::<Vec<_>>().join(" · ")));
        }
        if !html.is_empty() {
            return html;
        }
        if home {
            "<button class='empty compact' data-act='today'><img src='quiet.png' alt=''><span>Сегодня занятий нет</span></button>".into()
        } else {
            "<div class='empty large'><img src='day-off.png' alt=''><span>На этот день занятий нет</span></div>".into()
        }
    }
    fn schedule(&self) -> String {
        let start = monday(self.selected);
        let days = ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Вс"]
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let date = start + Duration::days(i as i64);
                button(
                    &format!("day:{i}"),
                    &format!("<small>{n}</small>{}", date.day()),
                    date == self.selected,
                )
            })
            .collect::<String>();
        let group = self
            .group
            .as_ref()
            .map(|g| {
                format!(
                    "<button class='group-name' data-act='groups'>{}</button>",
                    escape(&g.1)
                )
            })
            .unwrap_or_default();
        // Week parity is anchored at the Monday of the week containing September 1.
        let year = if u8::from(self.selected.month()) >= 9 {
            self.selected.year()
        } else {
            self.selected.year() - 1
        };
        let first = monday(Date::from_calendar_date(year, time::Month::September, 1).unwrap());
        let parity = (start - first).whole_weeks().rem_euclid(2) + 1;
        format!("<div class='days'>{days}</div>{group}<main class='scroll'>{}</main><div class='week'>{}<b>{parity} неделя - с {} по {}</b>{}</div>",
            self.lessons(self.selected,false),tool("week:prev","left","Предыдущая неделя"),short(start),short(start+Duration::days(6)),tool("week:next","right","Следующая неделя"))
    }
    fn course_notice(&self, t: &Term) -> String {
        t.courses_key
            .as_ref()
            .and_then(|key| self.status(key))
            .unwrap_or_else(|| "<div class='notice'>На этот семестр предметов нет</div>".into())
    }
    fn subjects(&self) -> String {
        if let Some(s) = self.status("studies") {
            return format!("<main class='scroll'>{s}</main>");
        }
        let terms = self.terms();
        let Some(t) = terms.get(self.term) else {
            return "<main class='notice'>Предметов пока нет</main>".into();
        };
        let mut years = String::new();
        let mut sems = String::new();
        for (i, other) in terms.iter().enumerate() {
            if i == 0
                || (terms[i - 1].year, &terms[i - 1].speciality) != (other.year, &other.speciality)
            {
                years.push_str(&button(
                    &format!("term:{i}"),
                    &format!("{} курс", other.year),
                    t.year == other.year && t.speciality == other.speciality,
                ))
            }
            if other.year == t.year && other.speciality == t.speciality {
                sems.push_str(&button(
                    &format!("term:{i}"),
                    &format!("{} семестр", other.number),
                    i == self.term,
                ))
            }
        }
        let mut rows = String::new();
        for (i, c) in t.courses.iter().enumerate() {
            let control = read(
                c,
                &[
                    "ControlType",
                    "ControlTypeName",
                    "AttestationType",
                    "ExamType",
                ],
            );
            let class = if control.to_lowercase().contains("экзамен") {
                "exam"
            } else if control.to_lowercase().contains("дифф") {
                "diff"
            } else {
                "pass"
            };
            let chip = if control.is_empty() {
                String::new()
            } else {
                format!("<span class='chip {class}'>{control}</span>")
            };
            let who = teacher(c);
            let by = if who.is_empty() {
                String::new()
            } else {
                format!("<small>{} {who}</small>", icon("teacher"))
            };
            let points = |keys: &[&str]| {
                keys.iter()
                    .find_map(|k| c[k].as_f64())
                    .map(|x| format!("{x:.1}"))
                    .unwrap_or_else(|| "—".into())
            };
            rows.push_str(&format!("<button class='subject-row' data-act='subject:{i}'><span><strong>{}</strong>{chip}{by}</span><span class='score'>{}<br>из<br>{}</span></button>",
            read(c,&["Name"]),points(&["Score","Points"]),points(&["MaxScore","MaxPoints"])));
        }
        if rows.is_empty() {
            rows = self.course_notice(t)
        }
        format!("<div class='pills years'>{years}</div><div class='pills'>{sems}</div><main class='scroll'>{rows}</main>")
    }
    fn services(&self) -> String {
        let rows=SERVICES.iter().enumerate().map(|(i,(name,txt,ic,color))|format!(
            "<button class='service' data-act='service:{i}'><span class='tile {color}'>{}</span><span>{name}<small>{txt}</small></span>{}</button>",icon(ic),icon("right"))).collect::<String>();
        format!("<main class='scroll'>{rows}</main>")
    }
    /// Count every marked lesson once across all current programmes. A successful
    /// empty list is complete, while a failed request can be retried on its own.
    fn profile_stats(&self, terms: &[Term]) -> (String, String, String) {
        if let Some(status) = self.status("studies") {
            return ("—".into(), "—".into(), status);
        }
        if !terms.iter().any(|term| term.current) {
            return (
                "—".into(),
                "—".into(),
                "<p class='grade-status'>Нет данных за текущий семестр</p>".into(),
            );
        }

        let mut keys = std::collections::HashSet::new();
        let mut loading_courses = false;
        let mut failed = false;
        for term in terms.iter().filter(|term| term.current) {
            if let Some(key) = &term.courses_key {
                match self.cache.get(key) {
                    Some(Ok(_)) => {}
                    Some(Err(_)) => failed = true,
                    None => loading_courses = true,
                }
            }
            keys.extend(
                term.courses
                    .iter()
                    .flat_map(type_ids)
                    .map(|id| term.grade_key(&id)),
            );
        }

        let total = keys.len();
        let (mut ready, mut points, mut marks, mut missed) = (0, 0, 0, 0);
        for key in keys {
            match self.cache.get(&key) {
                Some(Ok(rows)) => {
                    ready += 1;
                    for row in list(rows) {
                        if let Some(value) = mark(row) {
                            points += value;
                            marks += 1;
                        }
                        if attendance(row) == Some(0) {
                            missed += 1;
                        }
                    }
                }
                Some(Err(_)) => failed = true,
                None => {}
            }
        }
        let score = if marks == 0 {
            "—".into()
        } else {
            format!("{:.2}", f64::from(points) / f64::from(marks))
        };
        let complete = !failed && !loading_courses && ready == total;
        let absences = if ready > 0 || complete {
            missed.to_string()
        } else {
            "—".into()
        };
        let status = if failed {
            format!("<p class='grade-status' role='status'>Данные за семестр загружены частично. {}</p>", button("retry","Повторить",false))
        } else if loading_courses {
            "<p class='grade-status' role='status'>Загрузка предметов семестра…</p>".into()
        } else if ready < total {
            format!("<p class='grade-status' role='status'>Подсчёт за семестр · {ready} из {total} типов занятий</p>")
        } else {
            String::new()
        };
        (score, absences, status)
    }
    fn records(&self, term: &Term, c: &Value, show_attendance: bool) -> String {
        let mut lines = Vec::new();
        for id in type_ids(c) {
            let key = term.grade_key(&id);
            if let Some(s) = self.status(&key) {
                lines.push(s);
                continue;
            }
            for row in list(self.value(&key)) {
                let text = if show_attendance {
                    if attendance(row) != Some(0) {
                        continue;
                    }
                    "Пропуск".into()
                } else {
                    read(row, &["GradeText"])
                };
                if text.is_empty() {
                    continue;
                }
                lines.push(format!(
                    "<div class='record'><span>{text}</span><small>{}</small></div>",
                    read(row, &["Date"]).get(..10).unwrap_or("")
                ));
            }
        }
        if lines.is_empty() && !show_attendance {
            "<p class='muted'>Записей пока нет</p>".into()
        } else {
            lines.join("")
        }
    }
    fn service(&self, i: usize) -> String {
        if i < 2 {
            if let Some(s) = self.status("studies") {
                return s;
            }
            let terms = self.terms();
            let Some(t) = terms.get(self.term) else {
                return "<div class='notice'>Семестры пока недоступны</div>".into();
            };
            let chooser = terms
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    button(
                        &format!("term:{i}"),
                        &format!("{} сем.", t.number),
                        i == self.term,
                    )
                })
                .collect::<String>();
            let mut body = format!("<div class='pills years'>{chooser}</div>");
            if t.courses.is_empty() {
                body.push_str(&self.course_notice(t))
            }
            let mut sections = 0;
            for c in &t.courses {
                let records = self.records(t, c, i == 1);
                if records.is_empty() {
                    continue;
                }
                sections += 1;
                body.push_str(&format!(
                    "<article class='detail'><h2>{}</h2>{}</article>",
                    read(c, &["Name"]),
                    records
                ))
            }
            if i == 1 && sections == 0 && !t.courses.is_empty() {
                body.push_str("<p class='notice'>В этом семестре пропусков нет</p>");
            }
            return body;
        }
        let (_, _, icon_name, color) = SERVICES[i];
        format!(
            "<section class='unavailable'><span class='tile {color}'>{}</span><p>Сервис пока недоступен</p></section>",
            icon(icon_name)
        )
    }
    fn subject(&self, i: usize) -> String {
        let terms = self.terms();
        let Some((term, c)) = terms
            .get(self.term)
            .and_then(|t| t.courses.get(i).map(|c| (t, c)))
        else {
            return "<div class='notice'>Нет сведений о предмете</div>".into();
        };
        let absences = self.records(term, c, true);
        let absences = if absences.is_empty() {
            "<p class='muted'>Пропусков нет</p>"
        } else {
            &absences
        };
        format!("<article class='detail'><h2>{}</h2><p>{}</p><h3>Оценки</h3>{}<h3>Пропуски</h3>{absences}</article>",read(c,&["Name"]),teacher(c),self.records(term,c,false))
    }
    fn directory_view(&self) -> String {
        let (key, label, cmd) = match self.page {
            Page::Groups => ("institutes".into(), "Институт", "inst"),
            Page::ChooseSpec(i) => (format!("spec/{i}"), "Специальность", "spec"),
            Page::ChooseGroup(i, s) => (format!("groups/{i}/{s}"), "Группа", "group"),
            _ => return String::new(),
        };
        let mut out=format!("<article class='detail'><button class='link' data-act='personal'>Моё расписание</button><h2>{label}</h2>");
        if let Some(s) = self.status(&key) {
            out.push_str(&s)
        } else {
            let rows: Vec<_> = if let Page::ChooseGroup(i, s) = self.page {
                self.group_rows(i, s)
            } else {
                list(self.value(&key)).iter().collect()
            };
            if rows.is_empty() {
                out.push_str("<p>Список пуст</p>")
            }
            for (i, row) in rows.iter().enumerate() {
                out.push_str(&button(
                    &format!("{cmd}:{i}"),
                    &read(row, &["Name", "ShortName"]),
                    false,
                ))
            }
        }
        out.push_str("</article>");
        out
    }
}
