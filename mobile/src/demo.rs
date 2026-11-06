use serde_json::{json, Value};
use time::{Date, Duration};

pub fn profile() -> Value {
    json!({"Id":"demo","Name":"Анна Соколова","GroupName":"ПИЖ-б-о-23-1","SpecialtyName":"Программная инженерия (09.03.04)"})
}
pub fn studies() -> Value {
    let names = [
        "Администрирование баз данных",
        "Инженер-тестировщик",
        "Контейнерные технологии и DevOps для ИТ-специалистов",
        "Междисциплинарный проектный практикум",
        "Многопоточное и параллельное программирование",
        "Научно-исследовательская работа",
    ];
    let teachers = [
        "Андрей Маликов",
        "Регина Евдокимова",
        "Дмитрий Винокурский",
        "Анна Орлова",
        "Виктория Березина",
        "Анна Орлова",
    ];
    let controls = [
        "Зачет",
        "Экзамен",
        "Экзамен",
        "Дифференцированный зачет",
        "Зачет",
        "Дифференцированный зачет",
    ];
    let earlier = [
        "Высшая математика",
        "Алгоритмы и структуры данных",
        "Операционные системы",
        "Проектирование программного обеспечения",
        "Компьютерные сети",
        "Архитектура приложений",
    ];
    let mut years = Vec::new();
    for year in 0..4 {
        let mut terms = Vec::new();
        for offset in 0..2 {
            let i = year * 2 + offset;
            let labels: &[&str] = match i {
                6 => &names,
                7 => &[],
                _ => &earlier[i..=i],
            };
            let courses:Vec<_>=labels.iter().enumerate().map(|(j,name)|json!({"Name": name, "Teacher": teachers[j], "ControlType":controls[j],"Score":0.0,"MaxScore":0.0,"LessonTypes":[{"Id":i*10+j+1}]})).collect();
            terms.push(json!({"IsCurrent":i==6,"Courses":courses}));
        }
        years.push(json!({"Terms":terms}));
    }
    json!({"specialities":[{"Name":"Программная инженерия","AcademicYears":years}]})
}
pub fn week(start: Date) -> Value {
    let days:Vec<_>=(0..7).map(|i| {
        let date=start+Duration::days(i);
        let lessons=if i==0 || i==2 {
            json!([{"Discipline":"Администрирование баз данных","LessonType":"Лекция","TimeBegin":"09:40","TimeEnd":"11:10","Teacher":{"Name":"Андрей Маликов"},"Aud":{"Name":"2-314"},"Groups":[{"Name":"ПИЖ-б-о-23-1"}]}])
        } else {json!([])};
        json!({"Date":date.to_string(),"Lessons":lessons})
    }).collect();
    json!(days)
}
pub fn grades(id: &str) -> Value {
    let n = id.parse::<usize>().unwrap_or(0);
    json!([{"Attendance":1,"GradeText":if n%3==0 {"хорошо"} else {"отлично"}},
           {"Attendance":if n%2==0 {0} else {1}}])
}
