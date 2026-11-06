use crate::{data::*, demo};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use time::{Date, Duration};

pub enum Page {
    Tab,
    Service(usize),
    Subject(usize),
    Groups,
    ChooseSpec(usize),
    ChooseGroup(usize, usize),
}
pub struct App {
    pub today: Date,
    pub selected: Date,
    pub tab: usize,
    pub page: Page,
    pub profile: Value,
    pub demo: bool,
    pub captcha: Value,
    pub message: String,
    pub cache: HashMap<String, Result<Value, String>>,
    pub pending: HashSet<String>,
    pub term: usize,
    pub group: Option<(String, String)>,
    pub effects: Vec<Value>,
}
impl App {
    pub fn new(today: Date) -> Self {
        Self {
            today,
            selected: today,
            tab: 0,
            page: Page::Tab,
            profile: Value::Null,
            demo: false,
            captcha: Value::Null,
            message: String::new(),
            cache: HashMap::new(),
            pending: HashSet::new(),
            term: 0,
            group: None,
            effects: Vec::new(),
        }
    }
    pub fn signed(&self) -> bool {
        !self.profile.is_null()
    }
    pub fn value(&self, key: &str) -> &Value {
        self.cache
            .get(key)
            .and_then(|r| r.as_ref().ok())
            .unwrap_or(&Value::Null)
    }
    pub fn terms(&self) -> Vec<Term> {
        let mut terms = terms(self.value("studies"), &field(&self.profile, &["Id"]));
        for term in &mut terms {
            if let Some(key) = &term.courses_key {
                term.courses = list(self.value(key)).to_vec();
            }
        }
        terms
    }
    pub fn week_key(&self, date: Date, group: bool) -> String {
        let id = if group {
            self.group.as_ref().map(|g| g.0.as_str())
        } else {
            None
        };
        format!("week/{}/{}", monday(date), id.unwrap_or("me"))
    }
    fn load(&mut self, key: String, path: &str, form: Value, format: &str) {
        if self.cache.contains_key(&key) || !self.pending.insert(key.clone()) {
            return;
        }
        self.effects
            .push(json!({"key":key,"path":path,"form":form,"format":format}));
    }
    fn week(&mut self, date: Date, group: bool) {
        let key = self.week_key(date, group);
        if self.demo {
            self.cache
                .entry(key)
                .or_insert_with(|| Ok(demo::week(monday(date))));
            return;
        }
        let target = if group { self.group.as_ref() } else { None };
        let form = json!({"Id":target.map(|g|g.0.clone()).unwrap_or_else(||field(&self.profile,&["Id"])),
            "date":format!("{}T00:00:00.000Z",monday(date)), "targetType":if target.is_some(){2}else{4}});
        self.load(key, "/Schedule/GetSchedule", form, "array");
    }
    fn studies(&mut self) {
        if self.demo {
            self.cache.insert("studies".into(), Ok(demo::studies()));
            self.select_current();
            self.grades();
            return;
        }
        self.load("studies".into(), "/studies", Value::Null, "studies");
    }
    fn select_current(&mut self) {
        let terms = self.terms();
        self.term = terms
            .iter()
            .position(|t| t.current)
            .unwrap_or_else(|| terms.len().saturating_sub(1));
    }
    fn grades(&mut self) {
        let terms = self.terms();
        for (index, term) in terms.iter().enumerate() {
            if !(term.current || index == self.term) {
                continue;
            }
            if let Some(key) = &term.courses_key {
                if term.id.is_empty() {
                    self.cache.insert(
                        key.clone(),
                        Err("В ответе eCampus отсутствует ID семестра.".into()),
                    );
                    continue;
                }
                self.load(
                    key.clone(),
                    "/studies/GetCourses",
                    json!({"studentId":term.student_id,"termId":term.id}),
                    "courses",
                );
            }
            for id in term.courses.iter().flat_map(type_ids) {
                let key = term.grade_key(&id);
                if self.demo {
                    self.cache.insert(key, Ok(demo::grades(&id)));
                } else {
                    self.load(
                        key,
                        "/studies/GetLessons",
                        json!({"studentId":term.student_id,"lessonTypeId":id}),
                        "array",
                    );
                }
            }
        }
    }
    fn enter(&mut self, p: Value) {
        self.profile = p;
        self.captcha = Value::Null;
        self.message.clear();
        self.week(self.today, false);
        self.studies();
        self.details();
    }
    fn details(&mut self) {
        if !self.demo {
            self.load("details".into(), "/details", Value::Null, "details");
        }
    }
    fn challenge(&mut self) {
        self.captcha = Value::Null;
        self.load(
            "challenge".into(),
            "/account/login",
            Value::Null,
            "challenge",
        );
    }
    fn reset(&mut self, msg: &str) {
        *self = Self::new(self.today);
        self.message = msg.into();
        self.effects.push(json!({"key":"reset"}));
    }
    pub fn result(&mut self, key: &str, data: Value, error: Option<&str>) {
        if !self.pending.remove(key) {
            return;
        }
        if let Some(msg) = error {
            if msg == "expired" && key != "details" && (self.signed() || key == "resume") {
                self.reset("Сессия завершилась. Войдите снова.");
                return;
            }
            match key {
                "challenge" | "resume" => self.message = msg.into(),
                "login" => {
                    self.message = msg.into();
                    self.challenge()
                }
                _ => {
                    self.cache.insert(key.into(), Err(msg.into()));
                }
            }
            return;
        }
        match key {
            "challenge" => self.captcha = data,
            "login" | "resume" => self.enter(data),
            _ => {
                self.cache.insert(key.into(), Ok(data));
                if key == "studies" {
                    self.select_current();
                }
                if key == "studies" || key.starts_with("courses/") {
                    self.grades();
                }
            }
        }
    }
    fn institutes(&mut self) {
        self.page = Page::Groups;
        if self.demo {
            self.cache.insert(
                "institutes".into(),
                Ok(json!([{"Name":"Институт цифрового развития","Id":1}])),
            );
        } else {
            self.load("institutes".into(), "/schedule", Value::Null, "institutes")
        }
    }
    fn directory(&mut self, inst: usize, spec: Option<usize>) {
        let Some(row) = list(self.value("institutes")).get(inst) else {
            return;
        };
        let mut form = json!({"instituteId":row["Id"],"branchId":row["BranchId"]});
        let spec_key = format!("spec/{inst}");
        let (key, path) = if let Some(i) = spec {
            let Some(s) = list(self.value(&spec_key)).get(i) else {
                return;
            };
            form["specialty"] = s["Name"].clone();
            self.page = Page::ChooseGroup(inst, i);
            (format!("groups/{inst}/{i}"), "/schedule/GetAcademicGroups")
        } else {
            self.page = Page::ChooseSpec(inst);
            (spec_key, "/schedule/GetSpecialities")
        };
        if self.demo {
            let rows = if spec.is_some() {
                json!([{"Value":[{"Id":"demo-group","Name":"ПИЖ-б-о-23-1"}]}])
            } else {
                json!([{"Name":"Программная инженерия"}])
            };
            self.cache.insert(key, Ok(rows));
            return;
        }
        self.load(key, path, form, "array");
    }
    pub fn group_rows(&self, inst: usize, spec: usize) -> Vec<&Value> {
        list(self.value(&format!("groups/{inst}/{spec}")))
            .iter()
            .flat_map(|b| list(&b["Value"]))
            .collect()
    }
    pub fn action(&mut self, action: &str, fields: &Value) {
        let (cmd, arg) = action.split_once(':').unwrap_or((action, ""));
        let idx = arg.parse::<usize>().unwrap_or(0);
        if !self.signed() {
            match cmd {
                "resume" => self.load(
                    "resume".into(),
                    "/schedule/my/student",
                    Value::Null,
                    "profile",
                ),
                "challenge" => {
                    self.message.clear();
                    self.challenge()
                }
                "login" if self.pending.is_empty() => {
                    self.message.clear();
                    if self.captcha.is_null() {
                        self.challenge()
                    } else {
                        let form = json!({"Login":fields["login"],"Password":fields["password"],"Code":fields["code"],
                            "RememberMe":"false","__RequestVerificationToken":self.captcha["token"]});
                        self.load("login".into(), "/account/login", form, "login");
                    }
                }
                "demo" => {
                    *self = Self::new(self.today);
                    self.demo = true;
                    self.enter(demo::profile())
                }
                _ => {}
            }
            return;
        }
        match cmd {
            "tab" => {
                self.tab = idx.min(3);
                self.page = Page::Tab;
                if idx == 1 {
                    self.week(self.selected, true)
                }
            }
            "logout" => self.reset(""),
            "retry-details" => {
                self.cache.remove("details");
                self.details();
            }
            "today" => {
                self.selected = self.today;
                self.group = None;
                self.tab = 1;
                self.page = Page::Tab;
                self.week(self.today, false)
            }
            "week" => {
                let offset = if arg == "next" { 7 } else { -7 };
                self.selected += Duration::days(offset);
                self.week(self.selected, true)
            }
            "day" => self.selected = monday(self.selected) + Duration::days(idx.min(6) as i64),
            "term" => {
                self.term = idx;
                self.grades()
            }
            "subject" => self.page = Page::Subject(idx),
            "service" => self.page = Page::Service(idx),
            "back" => match self.page {
                Page::ChooseGroup(i, _) => self.page = Page::ChooseSpec(i),
                Page::ChooseSpec(_) => self.page = Page::Groups,
                Page::Tab if self.tab == 0 => self.effects.push(json!({"key":"close"})),
                Page::Tab => self.tab = 0,
                _ => self.page = Page::Tab,
            },
            "groups" => self.institutes(),
            "inst" => self.directory(idx, None),
            "spec" => {
                if let Page::ChooseSpec(i) = self.page {
                    self.directory(i, Some(idx))
                }
            }
            "group" => {
                if let Page::ChooseGroup(i, s) = self.page {
                    if let Some(row) = self.group_rows(i, s).get(idx) {
                        self.group = Some((field(row, &["Id"]), field(row, &["Name"])));
                        self.page = Page::Tab;
                        self.week(self.selected, true)
                    }
                }
            }
            "personal" => {
                self.group = None;
                self.page = Page::Tab;
                self.week(self.selected, false)
            }
            "retry" => {
                self.cache.retain(|key, v| key == "details" || v.is_ok());
                match self.page {
                    Page::Groups => self.institutes(),
                    Page::ChooseSpec(i) => self.directory(i, None),
                    Page::ChooseGroup(i, s) => self.directory(i, Some(s)),
                    _ => {
                        self.week(self.today, false);
                        self.week(self.selected, true);
                        self.studies();
                        self.grades()
                    }
                }
            }
            "refresh" => {
                self.cache.retain(|k, _| {
                    k != "studies" && !k.starts_with("grade/") && !k.starts_with("courses/")
                });
                self.studies()
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn initial() -> App {
        App::new(parse_date("2026-11-06").unwrap())
    }

    #[test]
    fn demo_navigation_stays_offline_and_separate_from_login() {
        let mut app = initial();
        for action in [
            "demo",
            "tab:1",
            "week:prev",
            "groups",
            "inst:0",
            "spec:0",
            "group:0",
            "tab:2",
            "term:7",
            "service:1",
        ] {
            app.action(action, &Value::Null);
            assert!(app.effects.is_empty());
            assert!(app.signed());
        }
        app.action("logout", &Value::Null);
        assert!(!app.signed());
        assert!(app.cache.is_empty());
        assert!(app.captcha.is_null());
        assert!(app.group.is_none());
    }
    #[test]
    fn profile_aggregates_current_programmes_and_retries_only_missing_data() {
        let mut app = initial();
        app.enter(json!({"Id":7}));
        app.effects.clear();
        let course = json!({"LessonTypes":[{"Id":81}]});
        let studies = json!({"specialities":[
            {"Id":101,"AcademicYears":[{"Terms":[
                {"IsCurrent":false,"Courses":[{"LessonTypes":[{"Id":12}]}]},
                {"IsCurrent":true,"Courses":[course,course]}
            ]}]},
            {"Id":202,"AcademicYears":[{"Terms":[{"Id":2,"IsCurrent":true}]}]}
        ]});
        app.result("studies", studies, None);
        assert_eq!(app.effects.len(), 2);
        let params = |jobs: &[Value], key: &str| {
            jobs.iter().find(|j| j["key"] == key).unwrap()["form"].clone()
        };
        assert_eq!(
            params(&app.effects, "grade/101/81"),
            json!({"studentId":"101","lessonTypeId":"81"})
        );
        assert_eq!(
            params(&app.effects, "courses/202/2"),
            json!({"studentId":"202","termId":"2"})
        );
        app.effects.clear();
        app.result("courses/202/2", json!([course]), None);
        assert_eq!(app.effects.len(), 1);
        assert_eq!(params(&app.effects, "grade/202/81")["studentId"], "202");
        app.result(
            "grade/101/81",
            json!([
                {"GradeText":"Отлично","Attendance":"0"}, {"GradeText":"5","Attendance":1},
                {"GradeText":"зачтено","Attendance":0}
            ]),
            None,
        );
        app.result("grade/202/81", Value::Null, Some("Ошибка сети"));
        let html = app.html();
        assert!(html.contains("<b>5.00</b>") && html.contains("<b>2</b>"));
        assert!(html.contains("загружены частично"));
        app.effects.clear();
        app.action("retry", &Value::Null);
        assert_eq!(app.effects.len(), 1);
        assert_eq!(app.effects[0]["key"], "grade/202/81");
        app.result(
            "grade/202/81",
            json!([{"GradeText":"удовлетворительно","Attendance":0}]),
            None,
        );
        let html = app.html();
        assert!(html.contains("<b>4.33</b>") && html.contains("<b>3</b>"));
        assert!(!html.contains("загружены частично"));
        // Looking at an old semester must not change the profile totals.
        app.action("term:0", &Value::Null);
        assert!(app.html().contains("<b>4.33</b>"));
    }

    #[test]
    fn profile_distinguishes_missing_grades_from_no_absences() {
        let mut app = initial();
        app.enter(json!({"Id":7}));
        app.result(
            "studies",
            json!({"specialities":[{"Id":101,"AcademicYears":[{
                "Terms":[{"IsCurrent":true,"Courses":[{"LessonTypes":[{"Id":18}]}]}]
            }]}]}),
            None,
        );
        assert!(app.html().contains("Подсчёт за семестр"));
        assert!(!app.html().contains("<b>0</b>"));
        app.result("grade/101/18", json!([]), None);
        let html = app.html();
        assert!(html.contains("<b>—</b><small>Средняя оценка"));
        assert!(html.contains("<b>0</b><small>Пропуски"));
    }
    #[test]
    fn attendance_lists_only_absences_and_waits_for_all_courses() {
        let mut app = initial();
        app.enter(json!({"Id":70}));
        app.result(
            "studies",
            json!({"specialities":[{"Id":101,"AcademicYears":[{"Terms":[{
                "IsCurrent":true,"Courses":[
                    {"Name":"Первый предмет","LessonTypes":[{"Id":11}]},
                    {"Name":"Второй предмет","LessonTypes":[{"Id":12}]}
                ]
            }]}]}]}),
            None,
        );
        app.action("service:1", &Value::Null);
        app.result(
            "grade/101/11",
            json!([{"Attendance":1},{"Attendance":null}]),
            None,
        );
        assert!(!app.html().contains("Первый предмет"));
        assert!(!app.html().contains("В этом семестре пропусков нет"));
        app.result("grade/101/12", json!([{"Attendance":1}]), None);
        assert!(app.html().contains("В этом семестре пропусков нет"));
        app.action("subject:0", &Value::Null);
        assert!(app.html().contains("Пропусков нет"));
        app.cache.insert(
            "grade/101/12".into(),
            Ok(json!([
                {"Attendance":"0","Date":"2026-10-14"}, {"Attendance":1,"Date":"2026-10-15"}
            ])),
        );
        for action in ["service:1", "subject:1"] {
            app.action(action, &Value::Null);
            let html = app.html();
            assert!(html.contains("2026-10-14") && !html.contains("2026-10-15"));
            assert_eq!(html.matches("class='record'").count(), 1);
        }
    }

    #[test]
    fn details_can_be_retried_without_replacing_the_schedule_identity() {
        let mut app = initial();
        app.enter(json!({"Id":42,"Name":"Профиль расписания","GroupName":"Группа 1"}));
        assert!(app
            .effects
            .iter()
            .any(|job| job["path"] == "/details" && job["form"].is_null()));
        app.result("details", Value::Null, Some("expired"));
        assert!(app.signed() && app.html().contains("Профиль расписания"));
        app.effects.clear();
        app.action("retry-details", &Value::Null);
        assert_eq!(app.effects.len(), 1);
        app.result(
            "details",
            json!({"FullName":"Полное имя","DisplayName":"Имя П.","Course":4,
            "SpecialtyName":"Программная инженерия (09.03.04)"}),
            None,
        );
        let html = app.html();
        for text in [
            "Полное имя",
            "Группа 1",
            "4 курс",
            "Программная инженерия (09.03.04)",
        ] {
            assert!(html.contains(text));
        }
        assert!(!html.contains("Имя П."));
        assert_eq!(app.profile["Id"], 42);
    }

    #[test]
    fn rejection_refreshes_token_and_keeps_user_on_login() {
        let mut app = initial();
        app.action("login", &Value::Null);
        app.result(
            "challenge",
            json!({"token":"token-a","image":"data:image/png;base64,AA=="}),
            None,
        );
        app.action(
            "login",
            &json!({"code":"42","login":"test","password":"password"}),
        );
        assert_eq!(
            app.effects.last().unwrap()["form"]["__RequestVerificationToken"],
            "token-a"
        );
        app.result("login", Value::Null, Some("Проверьте логин"));
        assert!(app.pending.contains("challenge"));
        assert!(app.captcha.is_null());
        assert!(!app.signed());
    }
    #[test]
    fn schedule_error_is_retriable_and_old_account_results_are_ignored() {
        let mut app = initial();
        app.enter(json!({"Id":42}));
        let key = app.week_key(app.today, false);
        app.result(&key, Value::Null, Some("Ошибка сети"));
        assert!(app.html().contains("Ошибка сети"));
        assert!(!app.html().contains("Сегодня занятий нет"));
        app.action("retry", &Value::Null);
        assert!(app.pending.contains(&key));
        app.result("studies", Value::Null, Some("expired"));
        app.result(&key, json!([]), None);
        assert!(app.cache.is_empty());
        assert!(!app.signed());
    }
}
