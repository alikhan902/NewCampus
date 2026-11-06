use serde_json::Value;
use time::{Date, Duration, Month};

pub fn list(value: &Value) -> &[Value] {
    value.as_array().map(Vec::as_slice).unwrap_or_default()
}
pub fn field(row: &Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| match &row[*key] {
            Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            Value::Object(o) => o.get("Name")?.as_str().map(str::to_owned),
            _ => None,
        })
        .unwrap_or_default()
}
pub fn parse_date(s: &str) -> Option<Date> {
    let mut parts = s.get(..10)?.split('-');
    Date::from_calendar_date(
        parts.next()?.parse().ok()?,
        Month::try_from(parts.next()?.parse::<u8>().ok()?).ok()?,
        parts.next()?.parse().ok()?,
    )
    .ok()
}
pub fn monday(d: Date) -> Date {
    d - Duration::days(d.weekday().number_days_from_monday() as i64)
}
pub fn short(d: Date) -> String {
    format!("{:02}.{:02}", d.day(), u8::from(d.month()))
}

pub struct Term {
    pub year: usize,
    pub number: usize,
    pub speciality: String,
    pub current: bool,
    pub student_id: String,
    pub id: String,
    pub courses_key: Option<String>,
    pub courses: Vec<Value>,
}
impl Term {
    pub fn grade_key(&self, lesson_type: &str) -> String {
        format!("grade/{}/{lesson_type}", self.student_id)
    }
}

pub fn terms(studies: &Value, fallback_id: &str) -> Vec<Term> {
    let mut out = Vec::new();
    for spec in list(&studies["specialities"]) {
        // eCampus sends speciality.Id as studentId in the studies section.
        let mut student_id = field(spec, &["Id"]);
        if student_id.is_empty() {
            student_id = fallback_id.to_owned();
        }
        for (y, year) in list(&spec["AcademicYears"]).iter().enumerate() {
            for (t, term) in list(&year["Terms"]).iter().enumerate() {
                let id = field(term, &["Id"]);
                let courses_key =
                    (!term["Courses"].is_array()).then(|| format!("courses/{student_id}/{id}"));
                out.push(Term {
                    year: y + 1,
                    number: y * 2 + t + 1,
                    speciality: field(spec, &["Name", "SpecialtyName"]),
                    current: matches!(term["IsCurrent"], Value::Bool(true))
                        || term["IsCurrent"].as_i64() == Some(1),
                    student_id: student_id.clone(),
                    id,
                    courses_key,
                    courses: list(&term["Courses"]).to_vec(),
                });
            }
        }
    }
    out
}
pub fn type_ids(course: &Value) -> impl Iterator<Item = String> + '_ {
    list(&course["LessonTypes"])
        .iter()
        .map(|x| field(x, &["Id"]))
        .filter(|id| !id.is_empty())
}
pub fn attendance(v: &Value) -> Option<i64> {
    let value = &v["Attendance"];
    value
        .as_i64()
        .or_else(|| value.as_str()?.trim().parse().ok())
}

pub fn mark(v: &Value) -> Option<u32> {
    match field(v, &["GradeText"]).to_lowercase().trim() {
        "отлично" | "5" => Some(5),
        "хорошо" | "4" => Some(4),
        "удовлетворительно" | "3" => Some(3),
        "неудовлетворительно" | "2" => Some(2),
        _ => None,
    }
}
pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn monday_wraps_year() {
        assert_eq!(
            monday(parse_date("2026-01-01").unwrap()).to_string(),
            "2025-12-29"
        );
    }
}
