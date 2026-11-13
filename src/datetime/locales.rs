//! Complete official d3-time-format 4.1.0 locale directory (34 locales).
//! Source: <https://github.com/d3/d3-time-format/tree/76cfef1da81b70404c0226ec9db9b0d56fa09461/locale>
//! Generated typed static data; no runtime filesystem/network access.
// Upstream ISC license follows:
// Copyright 2010-2021 Mike Bostock
//
// Permission to use, copy, modify, and/or distribute this software for any purpose
// with or without fee is hereby granted, provided that the above copyright notice
// and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
// REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND
// FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
// INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS
// OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER
// TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF
// THIS SOFTWARE.
use super::*;
pub const TIME_LOCALE_UPSTREAM_VERSION: &str = "d3-time-format 4.1.0";
pub const TIME_LOCALE_UPSTREAM_COMMIT: &str = "76cfef1da81b70404c0226ec9db9b0d56fa09461";
pub const SUPPORTED_TIME_LOCALES: &[&str] = &[
    "ar-EG", "ar-SY", "ca-ES", "cs-CZ", "da-DK", "de-CH", "de-DE", "en-CA", "en-GB", "en-US",
    "es-ES", "es-MX", "fa-IR", "fi-FI", "fr-CA", "fr-FR", "he-IL", "hr-HR", "hu-HU", "it-IT",
    "ja-JP", "ko-KR", "mk-MK", "nb-NO", "nl-BE", "nl-NL", "pl-PL", "pt-BR", "ru-RU", "sv-SE",
    "tr-TR", "uk-UA", "zh-CN", "zh-TW",
];
impl TimeLocale {
    pub const NAMES: &[&str] = SUPPORTED_TIME_LOCALES;
    pub fn for_name(name: &str) -> Result<Self> {
        let locale = match name {
            "ar-EG" => Self {
                date_time: "%x, %X".into(),
                date: "%-d/%-m/%Y".into(),
                time: "%-I:%M:%S %p".into(),
                periods: ["ص", "م"].map(str::to_string),
                days: [
                    "الأحد",
                    "الإثنين",
                    "الثلاثاء",
                    "الأربعاء",
                    "الخميس",
                    "الجمعة",
                    "السبت",
                ]
                .map(str::to_string),
                short_days: ["أحد", "إثنين", "ثلاثاء", "أربعاء", "خميس", "جمعة", "سبت"]
                    .map(str::to_string),
                months: [
                    "يناير",
                    "فبراير",
                    "مارس",
                    "أبريل",
                    "مايو",
                    "يونيو",
                    "يوليو",
                    "أغسطس",
                    "سبتمبر",
                    "أكتوبر",
                    "نوفمبر",
                    "ديسمبر",
                ]
                .map(str::to_string),
                short_months: [
                    "يناير",
                    "فبراير",
                    "مارس",
                    "أبريل",
                    "مايو",
                    "يونيو",
                    "يوليو",
                    "أغسطس",
                    "سبتمبر",
                    "أكتوبر",
                    "نوفمبر",
                    "ديسمبر",
                ]
                .map(str::to_string),
            },
            "ar-SY" => Self {
                date_time: "%x, %X".into(),
                date: "%-d/%-m/%Y".into(),
                time: "%-I:%M:%S %p".into(),
                periods: ["ص", "م"].map(str::to_string),
                days: [
                    "الأحد",
                    "الإثنين",
                    "الثلاثاء",
                    "الأربعاء",
                    "الخميس",
                    "الجمعة",
                    "السبت",
                ]
                .map(str::to_string),
                short_days: ["أحد", "إثنين", "ثلاثاء", "أربعاء", "خميس", "جمعة", "سبت"]
                    .map(str::to_string),
                months: [
                    "كانون الثاني",
                    "شباط",
                    "آذار",
                    "نيسان",
                    "أيار",
                    "حزيران",
                    "تموز",
                    "آب",
                    "أيلول",
                    "تشرين الأول",
                    "تشرين الثاني",
                    "كانون الأول",
                ]
                .map(str::to_string),
                short_months: [
                    "ك٢",
                    "شباط",
                    "آذار",
                    "نيسان",
                    "أيار",
                    "حزيران",
                    "تموز",
                    "آب",
                    "أيلول",
                    "ت١",
                    "ت٢",
                    "ك١",
                ]
                .map(str::to_string),
            },
            "ca-ES" => Self {
                date_time: "%A, %e de %B de %Y, %X".into(),
                date: "%d/%m/%Y".into(),
                time: "%H:%M:%S".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: [
                    "diumenge",
                    "dilluns",
                    "dimarts",
                    "dimecres",
                    "dijous",
                    "divendres",
                    "dissabte",
                ]
                .map(str::to_string),
                short_days: ["dg.", "dl.", "dt.", "dc.", "dj.", "dv.", "ds."].map(str::to_string),
                months: [
                    "gener", "febrer", "març", "abril", "maig", "juny", "juliol", "agost",
                    "setembre", "octubre", "novembre", "desembre",
                ]
                .map(str::to_string),
                short_months: [
                    "gen.", "febr.", "març", "abr.", "maig", "juny", "jul.", "ag.", "set.", "oct.",
                    "nov.", "des.",
                ]
                .map(str::to_string),
            },
            "cs-CZ" => Self {
                date_time: "%A,%e.%B %Y, %X".into(),
                date: "%-d.%-m.%Y".into(),
                time: "%H:%M:%S".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: [
                    "neděle",
                    "pondělí",
                    "úterý",
                    "středa",
                    "čvrtek",
                    "pátek",
                    "sobota",
                ]
                .map(str::to_string),
                short_days: ["ne.", "po.", "út.", "st.", "čt.", "pá.", "so."].map(str::to_string),
                months: [
                    "leden",
                    "únor",
                    "březen",
                    "duben",
                    "květen",
                    "červen",
                    "červenec",
                    "srpen",
                    "září",
                    "říjen",
                    "listopad",
                    "prosinec",
                ]
                .map(str::to_string),
                short_months: [
                    "led", "úno", "břez", "dub", "kvě", "čer", "červ", "srp", "zář", "říj", "list",
                    "pros",
                ]
                .map(str::to_string),
            },
            "da-DK" => Self {
                date_time: "%A den %d %B %Y %X".into(),
                date: "%d-%m-%Y".into(),
                time: "%H:%M:%S".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: [
                    "søndag", "mandag", "tirsdag", "onsdag", "torsdag", "fredag", "lørdag",
                ]
                .map(str::to_string),
                short_days: ["søn", "man", "tir", "ons", "tor", "fre", "lør"].map(str::to_string),
                months: [
                    "januar",
                    "februar",
                    "marts",
                    "april",
                    "maj",
                    "juni",
                    "juli",
                    "august",
                    "september",
                    "oktober",
                    "november",
                    "december",
                ]
                .map(str::to_string),
                short_months: [
                    "jan", "feb", "mar", "apr", "maj", "jun", "jul", "aug", "sep", "okt", "nov",
                    "dec",
                ]
                .map(str::to_string),
            },
            "de-CH" => Self {
                date_time: "%A, der %e. %B %Y, %X".into(),
                date: "%d.%m.%Y".into(),
                time: "%H:%M:%S".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: [
                    "Sonntag",
                    "Montag",
                    "Dienstag",
                    "Mittwoch",
                    "Donnerstag",
                    "Freitag",
                    "Samstag",
                ]
                .map(str::to_string),
                short_days: ["So", "Mo", "Di", "Mi", "Do", "Fr", "Sa"].map(str::to_string),
                months: [
                    "Januar",
                    "Februar",
                    "März",
                    "April",
                    "Mai",
                    "Juni",
                    "Juli",
                    "August",
                    "September",
                    "Oktober",
                    "November",
                    "Dezember",
                ]
                .map(str::to_string),
                short_months: [
                    "Jan", "Feb", "Mrz", "Apr", "Mai", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov",
                    "Dez",
                ]
                .map(str::to_string),
            },
            "de-DE" => Self {
                date_time: "%A, der %e. %B %Y, %X".into(),
                date: "%d.%m.%Y".into(),
                time: "%H:%M:%S".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: [
                    "Sonntag",
                    "Montag",
                    "Dienstag",
                    "Mittwoch",
                    "Donnerstag",
                    "Freitag",
                    "Samstag",
                ]
                .map(str::to_string),
                short_days: ["So", "Mo", "Di", "Mi", "Do", "Fr", "Sa"].map(str::to_string),
                months: [
                    "Januar",
                    "Februar",
                    "März",
                    "April",
                    "Mai",
                    "Juni",
                    "Juli",
                    "August",
                    "September",
                    "Oktober",
                    "November",
                    "Dezember",
                ]
                .map(str::to_string),
                short_months: [
                    "Jan", "Feb", "Mrz", "Apr", "Mai", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov",
                    "Dez",
                ]
                .map(str::to_string),
            },
            "en-CA" => Self {
                date_time: "%a %b %e %X %Y".into(),
                date: "%Y-%m-%d".into(),
                time: "%H:%M:%S".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: [
                    "Sunday",
                    "Monday",
                    "Tuesday",
                    "Wednesday",
                    "Thursday",
                    "Friday",
                    "Saturday",
                ]
                .map(str::to_string),
                short_days: ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"].map(str::to_string),
                months: [
                    "January",
                    "February",
                    "March",
                    "April",
                    "May",
                    "June",
                    "July",
                    "August",
                    "September",
                    "October",
                    "November",
                    "December",
                ]
                .map(str::to_string),
                short_months: [
                    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov",
                    "Dec",
                ]
                .map(str::to_string),
            },
            "en-GB" => Self {
                date_time: "%a %e %b %X %Y".into(),
                date: "%d/%m/%Y".into(),
                time: "%H:%M:%S".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: [
                    "Sunday",
                    "Monday",
                    "Tuesday",
                    "Wednesday",
                    "Thursday",
                    "Friday",
                    "Saturday",
                ]
                .map(str::to_string),
                short_days: ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"].map(str::to_string),
                months: [
                    "January",
                    "February",
                    "March",
                    "April",
                    "May",
                    "June",
                    "July",
                    "August",
                    "September",
                    "October",
                    "November",
                    "December",
                ]
                .map(str::to_string),
                short_months: [
                    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov",
                    "Dec",
                ]
                .map(str::to_string),
            },
            "en-US" => Self {
                date_time: "%x, %X".into(),
                date: "%-m/%-d/%Y".into(),
                time: "%-I:%M:%S %p".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: [
                    "Sunday",
                    "Monday",
                    "Tuesday",
                    "Wednesday",
                    "Thursday",
                    "Friday",
                    "Saturday",
                ]
                .map(str::to_string),
                short_days: ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"].map(str::to_string),
                months: [
                    "January",
                    "February",
                    "March",
                    "April",
                    "May",
                    "June",
                    "July",
                    "August",
                    "September",
                    "October",
                    "November",
                    "December",
                ]
                .map(str::to_string),
                short_months: [
                    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov",
                    "Dec",
                ]
                .map(str::to_string),
            },
            "es-ES" => Self {
                date_time: "%A, %e de %B de %Y, %X".into(),
                date: "%d/%m/%Y".into(),
                time: "%H:%M:%S".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: [
                    "domingo",
                    "lunes",
                    "martes",
                    "miércoles",
                    "jueves",
                    "viernes",
                    "sábado",
                ]
                .map(str::to_string),
                short_days: ["dom", "lun", "mar", "mié", "jue", "vie", "sáb"].map(str::to_string),
                months: [
                    "enero",
                    "febrero",
                    "marzo",
                    "abril",
                    "mayo",
                    "junio",
                    "julio",
                    "agosto",
                    "septiembre",
                    "octubre",
                    "noviembre",
                    "diciembre",
                ]
                .map(str::to_string),
                short_months: [
                    "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov",
                    "dic",
                ]
                .map(str::to_string),
            },
            "es-MX" => Self {
                date_time: "%x, %X".into(),
                date: "%d/%m/%Y".into(),
                time: "%-I:%M:%S %p".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: [
                    "domingo",
                    "lunes",
                    "martes",
                    "miércoles",
                    "jueves",
                    "viernes",
                    "sábado",
                ]
                .map(str::to_string),
                short_days: ["dom", "lun", "mar", "mié", "jue", "vie", "sáb"].map(str::to_string),
                months: [
                    "enero",
                    "febrero",
                    "marzo",
                    "abril",
                    "mayo",
                    "junio",
                    "julio",
                    "agosto",
                    "septiembre",
                    "octubre",
                    "noviembre",
                    "diciembre",
                ]
                .map(str::to_string),
                short_months: [
                    "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov",
                    "dic",
                ]
                .map(str::to_string),
            },
            "fa-IR" => Self {
                date_time: "%x, %X".into(),
                date: "%-d/%-m/%Y".into(),
                time: "%-I:%M:%S %p".into(),
                periods: ["صبح", "عصر"].map(str::to_string),
                days: [
                    "یکشنبه",
                    "دوشنبه",
                    "سه شنبه",
                    "چهارشنبه",
                    "پنجشنبه",
                    "جمعه",
                    "شنبه",
                ]
                .map(str::to_string),
                short_days: [
                    "یکشنبه",
                    "دوشنبه",
                    "سه شنبه",
                    "چهارشنبه",
                    "پنجشنبه",
                    "جمعه",
                    "شنبه",
                ]
                .map(str::to_string),
                months: [
                    "ژانویه",
                    "فوریه",
                    "مارس",
                    "آوریل",
                    "مه",
                    "ژوئن",
                    "ژوئیه",
                    "اوت",
                    "سپتامبر",
                    "اکتبر",
                    "نوامبر",
                    "دسامبر",
                ]
                .map(str::to_string),
                short_months: [
                    "ژانویه",
                    "فوریه",
                    "مارس",
                    "آوریل",
                    "مه",
                    "ژوئن",
                    "ژوئیه",
                    "اوت",
                    "سپتامبر",
                    "اکتبر",
                    "نوامبر",
                    "دسامبر",
                ]
                .map(str::to_string),
            },
            "fi-FI" => Self {
                date_time: "%A, %-d. %Bta %Y klo %X".into(),
                date: "%-d.%-m.%Y".into(),
                time: "%H:%M:%S".into(),
                periods: ["a.m.", "p.m."].map(str::to_string),
                days: [
                    "sunnuntai",
                    "maanantai",
                    "tiistai",
                    "keskiviikko",
                    "torstai",
                    "perjantai",
                    "lauantai",
                ]
                .map(str::to_string),
                short_days: ["Su", "Ma", "Ti", "Ke", "To", "Pe", "La"].map(str::to_string),
                months: [
                    "tammikuu",
                    "helmikuu",
                    "maaliskuu",
                    "huhtikuu",
                    "toukokuu",
                    "kesäkuu",
                    "heinäkuu",
                    "elokuu",
                    "syyskuu",
                    "lokakuu",
                    "marraskuu",
                    "joulukuu",
                ]
                .map(str::to_string),
                short_months: [
                    "Tammi", "Helmi", "Maalis", "Huhti", "Touko", "Kesä", "Heinä", "Elo", "Syys",
                    "Loka", "Marras", "Joulu",
                ]
                .map(str::to_string),
            },
            "fr-CA" => Self {
                date_time: "%a %e %b %Y %X".into(),
                date: "%Y-%m-%d".into(),
                time: "%H:%M:%S".into(),
                periods: ["", ""].map(str::to_string),
                days: [
                    "dimanche", "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi",
                ]
                .map(str::to_string),
                short_days: ["dim", "lun", "mar", "mer", "jeu", "ven", "sam"].map(str::to_string),
                months: [
                    "janvier",
                    "février",
                    "mars",
                    "avril",
                    "mai",
                    "juin",
                    "juillet",
                    "août",
                    "septembre",
                    "octobre",
                    "novembre",
                    "décembre",
                ]
                .map(str::to_string),
                short_months: [
                    "jan", "fév", "mar", "avr", "mai", "jui", "jul", "aoû", "sep", "oct", "nov",
                    "déc",
                ]
                .map(str::to_string),
            },
            "fr-FR" => Self {
                date_time: "%A %e %B %Y à %X".into(),
                date: "%d/%m/%Y".into(),
                time: "%H:%M:%S".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: [
                    "dimanche", "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi",
                ]
                .map(str::to_string),
                short_days: ["dim.", "lun.", "mar.", "mer.", "jeu.", "ven.", "sam."]
                    .map(str::to_string),
                months: [
                    "janvier",
                    "février",
                    "mars",
                    "avril",
                    "mai",
                    "juin",
                    "juillet",
                    "août",
                    "septembre",
                    "octobre",
                    "novembre",
                    "décembre",
                ]
                .map(str::to_string),
                short_months: [
                    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.",
                    "oct.", "nov.", "déc.",
                ]
                .map(str::to_string),
            },
            "he-IL" => Self {
                date_time: "%A, %e ב%B %Y %X".into(),
                date: "%d.%m.%Y".into(),
                time: "%H:%M:%S".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: ["ראשון", "שני", "שלישי", "רביעי", "חמישי", "שישי", "שבת"]
                    .map(str::to_string),
                short_days: ["א׳", "ב׳", "ג׳", "ד׳", "ה׳", "ו׳", "ש׳"].map(str::to_string),
                months: [
                    "ינואר",
                    "פברואר",
                    "מרץ",
                    "אפריל",
                    "מאי",
                    "יוני",
                    "יולי",
                    "אוגוסט",
                    "ספטמבר",
                    "אוקטובר",
                    "נובמבר",
                    "דצמבר",
                ]
                .map(str::to_string),
                short_months: [
                    "ינו׳", "פבר׳", "מרץ", "אפר׳", "מאי", "יוני", "יולי", "אוג׳", "ספט׳", "אוק׳",
                    "נוב׳", "דצמ׳",
                ]
                .map(str::to_string),
            },
            "hr-HR" => Self {
                date_time: "%A, %e. %B %Y., %X".into(),
                date: "%d. %m. %Y.".into(),
                time: "%H:%M:%S".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: [
                    "Nedjelja",
                    "Ponedjeljak",
                    "Utorak",
                    "Srijeda",
                    "Četvtrak",
                    "Petak",
                    "Subota",
                ]
                .map(str::to_string),
                short_days: ["Ne", "Po", "Ut", "Sr", "Če", "Pe", "Su"].map(str::to_string),
                months: [
                    "Siječanj",
                    "Veljača",
                    "Ožujak",
                    "Travanj",
                    "Svibanj",
                    "Lipanj",
                    "Srpanj",
                    "Kolovoz",
                    "Rujan",
                    "Listopad",
                    "Studeni",
                    "Prosinac",
                ]
                .map(str::to_string),
                short_months: [
                    "Sij", "Velj", "Ožu", "Tra", "Svi", "Lip", "Srp", "Kol", "Ruj", "Lis", "Stu",
                    "Pro",
                ]
                .map(str::to_string),
            },
            "hu-HU" => Self {
                date_time: "%Y. %B %-e., %A %X".into(),
                date: "%Y. %m. %d.".into(),
                time: "%H:%M:%S".into(),
                periods: ["de.", "du."].map(str::to_string),
                days: [
                    "vasárnap",
                    "hétfő",
                    "kedd",
                    "szerda",
                    "csütörtök",
                    "péntek",
                    "szombat",
                ]
                .map(str::to_string),
                short_days: ["V", "H", "K", "Sze", "Cs", "P", "Szo"].map(str::to_string),
                months: [
                    "január",
                    "február",
                    "március",
                    "április",
                    "május",
                    "június",
                    "július",
                    "augusztus",
                    "szeptember",
                    "október",
                    "november",
                    "december",
                ]
                .map(str::to_string),
                short_months: [
                    "jan.", "feb.", "már.", "ápr.", "máj.", "jún.", "júl.", "aug.", "szept.",
                    "okt.", "nov.", "dec.",
                ]
                .map(str::to_string),
            },
            "it-IT" => Self {
                date_time: "%A %e %B %Y, %X".into(),
                date: "%d/%m/%Y".into(),
                time: "%H:%M:%S".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: [
                    "Domenica",
                    "Lunedì",
                    "Martedì",
                    "Mercoledì",
                    "Giovedì",
                    "Venerdì",
                    "Sabato",
                ]
                .map(str::to_string),
                short_days: ["Dom", "Lun", "Mar", "Mer", "Gio", "Ven", "Sab"].map(str::to_string),
                months: [
                    "Gennaio",
                    "Febbraio",
                    "Marzo",
                    "Aprile",
                    "Maggio",
                    "Giugno",
                    "Luglio",
                    "Agosto",
                    "Settembre",
                    "Ottobre",
                    "Novembre",
                    "Dicembre",
                ]
                .map(str::to_string),
                short_months: [
                    "Gen", "Feb", "Mar", "Apr", "Mag", "Giu", "Lug", "Ago", "Set", "Ott", "Nov",
                    "Dic",
                ]
                .map(str::to_string),
            },
            "ja-JP" => Self {
                date_time: "%x %a %X".into(),
                date: "%Y/%m/%d".into(),
                time: "%H:%M:%S".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: [
                    "日曜日",
                    "月曜日",
                    "火曜日",
                    "水曜日",
                    "木曜日",
                    "金曜日",
                    "土曜日",
                ]
                .map(str::to_string),
                short_days: ["日", "月", "火", "水", "木", "金", "土"].map(str::to_string),
                months: [
                    "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月",
                    "12月",
                ]
                .map(str::to_string),
                short_months: [
                    "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月",
                    "12月",
                ]
                .map(str::to_string),
            },
            "ko-KR" => Self {
                date_time: "%Y/%m/%d %a %X".into(),
                date: "%Y/%m/%d".into(),
                time: "%H:%M:%S".into(),
                periods: ["오전", "오후"].map(str::to_string),
                days: [
                    "일요일",
                    "월요일",
                    "화요일",
                    "수요일",
                    "목요일",
                    "금요일",
                    "토요일",
                ]
                .map(str::to_string),
                short_days: ["일", "월", "화", "수", "목", "금", "토"].map(str::to_string),
                months: [
                    "1월", "2월", "3월", "4월", "5월", "6월", "7월", "8월", "9월", "10월", "11월",
                    "12월",
                ]
                .map(str::to_string),
                short_months: [
                    "1월", "2월", "3월", "4월", "5월", "6월", "7월", "8월", "9월", "10월", "11월",
                    "12월",
                ]
                .map(str::to_string),
            },
            "mk-MK" => Self {
                date_time: "%A, %e %B %Y г. %X".into(),
                date: "%d.%m.%Y".into(),
                time: "%H:%M:%S".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: [
                    "недела",
                    "понеделник",
                    "вторник",
                    "среда",
                    "четврток",
                    "петок",
                    "сабота",
                ]
                .map(str::to_string),
                short_days: ["нед", "пон", "вто", "сре", "чет", "пет", "саб"].map(str::to_string),
                months: [
                    "јануари",
                    "февруари",
                    "март",
                    "април",
                    "мај",
                    "јуни",
                    "јули",
                    "август",
                    "септември",
                    "октомври",
                    "ноември",
                    "декември",
                ]
                .map(str::to_string),
                short_months: [
                    "јан", "фев", "мар", "апр", "мај", "јун", "јул", "авг", "сеп", "окт", "ное",
                    "дек",
                ]
                .map(str::to_string),
            },
            "nb-NO" => Self {
                date_time: "%A den %d. %B %Y %X".into(),
                date: "%d.%m.%Y".into(),
                time: "%H:%M:%S".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: [
                    "søndag", "mandag", "tirsdag", "onsdag", "torsdag", "fredag", "lørdag",
                ]
                .map(str::to_string),
                short_days: ["søn", "man", "tir", "ons", "tor", "fre", "lør"].map(str::to_string),
                months: [
                    "januar",
                    "februar",
                    "mars",
                    "april",
                    "mai",
                    "juni",
                    "juli",
                    "august",
                    "september",
                    "oktober",
                    "november",
                    "desember",
                ]
                .map(str::to_string),
                short_months: [
                    "jan", "feb", "mars", "apr", "mai", "juni", "juli", "aug", "sep", "okt", "nov",
                    "des",
                ]
                .map(str::to_string),
            },
            "nl-BE" => Self {
                date_time: "%a %e %B %Y %X".into(),
                date: "%d/%m/%Y".into(),
                time: "%H:%M:%S".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: [
                    "zondag",
                    "maandag",
                    "dinsdag",
                    "woensdag",
                    "donderdag",
                    "vrijdag",
                    "zaterdag",
                ]
                .map(str::to_string),
                short_days: ["zo", "ma", "di", "wo", "do", "vr", "za"].map(str::to_string),
                months: [
                    "januari",
                    "februari",
                    "maart",
                    "april",
                    "mei",
                    "juni",
                    "juli",
                    "augustus",
                    "september",
                    "oktober",
                    "november",
                    "december",
                ]
                .map(str::to_string),
                short_months: [
                    "jan", "feb", "mrt", "apr", "mei", "jun", "jul", "aug", "sep", "okt", "nov",
                    "dec",
                ]
                .map(str::to_string),
            },
            "nl-NL" => Self {
                date_time: "%a %e %B %Y %X".into(),
                date: "%d-%m-%Y".into(),
                time: "%H:%M:%S".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: [
                    "zondag",
                    "maandag",
                    "dinsdag",
                    "woensdag",
                    "donderdag",
                    "vrijdag",
                    "zaterdag",
                ]
                .map(str::to_string),
                short_days: ["zo", "ma", "di", "wo", "do", "vr", "za"].map(str::to_string),
                months: [
                    "januari",
                    "februari",
                    "maart",
                    "april",
                    "mei",
                    "juni",
                    "juli",
                    "augustus",
                    "september",
                    "oktober",
                    "november",
                    "december",
                ]
                .map(str::to_string),
                short_months: [
                    "jan", "feb", "mrt", "apr", "mei", "jun", "jul", "aug", "sep", "okt", "nov",
                    "dec",
                ]
                .map(str::to_string),
            },
            "pl-PL" => Self {
                date_time: "%A, %e %B %Y, %X".into(),
                date: "%d/%m/%Y".into(),
                time: "%H:%M:%S".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: [
                    "Niedziela",
                    "Poniedziałek",
                    "Wtorek",
                    "Środa",
                    "Czwartek",
                    "Piątek",
                    "Sobota",
                ]
                .map(str::to_string),
                short_days: ["Niedz.", "Pon.", "Wt.", "Śr.", "Czw.", "Pt.", "Sob."]
                    .map(str::to_string),
                months: [
                    "Styczeń",
                    "Luty",
                    "Marzec",
                    "Kwiecień",
                    "Maj",
                    "Czerwiec",
                    "Lipiec",
                    "Sierpień",
                    "Wrzesień",
                    "Październik",
                    "Listopad",
                    "Grudzień",
                ]
                .map(str::to_string),
                short_months: [
                    "Stycz.", "Luty", "Marz.", "Kwie.", "Maj", "Czerw.", "Lipc.", "Sierp.", "Wrz.",
                    "Paźdz.", "Listop.", "Grudz.",
                ]
                .map(str::to_string),
            },
            "pt-BR" => Self {
                date_time: "%A, %e de %B de %Y. %X".into(),
                date: "%d/%m/%Y".into(),
                time: "%H:%M:%S".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: [
                    "Domingo", "Segunda", "Terça", "Quarta", "Quinta", "Sexta", "Sábado",
                ]
                .map(str::to_string),
                short_days: ["Dom", "Seg", "Ter", "Qua", "Qui", "Sex", "Sáb"].map(str::to_string),
                months: [
                    "Janeiro",
                    "Fevereiro",
                    "Março",
                    "Abril",
                    "Maio",
                    "Junho",
                    "Julho",
                    "Agosto",
                    "Setembro",
                    "Outubro",
                    "Novembro",
                    "Dezembro",
                ]
                .map(str::to_string),
                short_months: [
                    "Jan", "Fev", "Mar", "Abr", "Mai", "Jun", "Jul", "Ago", "Set", "Out", "Nov",
                    "Dez",
                ]
                .map(str::to_string),
            },
            "ru-RU" => Self {
                date_time: "%A, %e %B %Y г. %X".into(),
                date: "%d.%m.%Y".into(),
                time: "%H:%M:%S".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: [
                    "воскресенье",
                    "понедельник",
                    "вторник",
                    "среда",
                    "четверг",
                    "пятница",
                    "суббота",
                ]
                .map(str::to_string),
                short_days: ["вс", "пн", "вт", "ср", "чт", "пт", "сб"].map(str::to_string),
                months: [
                    "января",
                    "февраля",
                    "марта",
                    "апреля",
                    "мая",
                    "июня",
                    "июля",
                    "августа",
                    "сентября",
                    "октября",
                    "ноября",
                    "декабря",
                ]
                .map(str::to_string),
                short_months: [
                    "янв", "фев", "мар", "апр", "май", "июн", "июл", "авг", "сен", "окт", "ноя",
                    "дек",
                ]
                .map(str::to_string),
            },
            "sv-SE" => Self {
                date_time: "%A den %d %B %Y %X".into(),
                date: "%Y-%m-%d".into(),
                time: "%H:%M:%S".into(),
                periods: ["fm", "em"].map(str::to_string),
                days: [
                    "Söndag", "Måndag", "Tisdag", "Onsdag", "Torsdag", "Fredag", "Lördag",
                ]
                .map(str::to_string),
                short_days: ["Sön", "Mån", "Tis", "Ons", "Tor", "Fre", "Lör"].map(str::to_string),
                months: [
                    "Januari",
                    "Februari",
                    "Mars",
                    "April",
                    "Maj",
                    "Juni",
                    "Juli",
                    "Augusti",
                    "September",
                    "Oktober",
                    "November",
                    "December",
                ]
                .map(str::to_string),
                short_months: [
                    "Jan", "Feb", "Mar", "Apr", "Maj", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov",
                    "Dec",
                ]
                .map(str::to_string),
            },
            "tr-TR" => Self {
                date_time: "%a %e %b %X %Y".into(),
                date: "%d/%m/%Y".into(),
                time: "%H:%M:%S".into(),
                periods: ["AM", "PM"].map(str::to_string),
                days: [
                    "Pazar",
                    "Pazartesi",
                    "Salı",
                    "Çarşamba",
                    "Perşembe",
                    "Cuma",
                    "Cumartesi",
                ]
                .map(str::to_string),
                short_days: ["Paz", "Pzt", "Sal", "Çar", "Per", "Cum", "Cmt"].map(str::to_string),
                months: [
                    "Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos",
                    "Eylül", "Ekim", "Kasım", "Aralık",
                ]
                .map(str::to_string),
                short_months: [
                    "Oca", "Şub", "Mar", "Nis", "May", "Haz", "Tem", "Ağu", "Eyl", "Eki", "Kas",
                    "Ara",
                ]
                .map(str::to_string),
            },
            "uk-UA" => Self {
                date_time: "%A, %e %B %Y р. %X".into(),
                date: "%d.%m.%Y".into(),
                time: "%H:%M:%S".into(),
                periods: ["дп", "пп"].map(str::to_string),
                days: [
                    "неділя",
                    "понеділок",
                    "вівторок",
                    "середа",
                    "четвер",
                    "п'ятниця",
                    "субота",
                ]
                .map(str::to_string),
                short_days: ["нд", "пн", "вт", "ср", "чт", "пт", "сб"].map(str::to_string),
                months: [
                    "січня",
                    "лютого",
                    "березня",
                    "квітня",
                    "травня",
                    "червня",
                    "липня",
                    "серпня",
                    "вересня",
                    "жовтня",
                    "листопада",
                    "грудня",
                ]
                .map(str::to_string),
                short_months: [
                    "січ.",
                    "лют.",
                    "бер.",
                    "квіт.",
                    "трав.",
                    "черв.",
                    "лип.",
                    "серп.",
                    "вер.",
                    "жовт.",
                    "лист.",
                    "груд.",
                ]
                .map(str::to_string),
            },
            "zh-CN" => Self {
                date_time: "%x %A %X".into(),
                date: "%Y年%-m月%-d日".into(),
                time: "%H:%M:%S".into(),
                periods: ["上午", "下午"].map(str::to_string),
                days: [
                    "星期日",
                    "星期一",
                    "星期二",
                    "星期三",
                    "星期四",
                    "星期五",
                    "星期六",
                ]
                .map(str::to_string),
                short_days: ["周日", "周一", "周二", "周三", "周四", "周五", "周六"]
                    .map(str::to_string),
                months: [
                    "一月",
                    "二月",
                    "三月",
                    "四月",
                    "五月",
                    "六月",
                    "七月",
                    "八月",
                    "九月",
                    "十月",
                    "十一月",
                    "十二月",
                ]
                .map(str::to_string),
                short_months: [
                    "一月",
                    "二月",
                    "三月",
                    "四月",
                    "五月",
                    "六月",
                    "七月",
                    "八月",
                    "九月",
                    "十月",
                    "十一月",
                    "十二月",
                ]
                .map(str::to_string),
            },
            "zh-TW" => Self {
                date_time: "%x %A %X".into(),
                date: "%Y年%-m月%-d日".into(),
                time: "%H:%M:%S".into(),
                periods: ["上午", "下午"].map(str::to_string),
                days: [
                    "星期日",
                    "星期一",
                    "星期二",
                    "星期三",
                    "星期四",
                    "星期五",
                    "星期六",
                ]
                .map(str::to_string),
                short_days: ["日", "一", "二", "三", "四", "五", "六"].map(str::to_string),
                months: [
                    "一月",
                    "二月",
                    "三月",
                    "四月",
                    "五月",
                    "六月",
                    "七月",
                    "八月",
                    "九月",
                    "十月",
                    "十一月",
                    "十二月",
                ]
                .map(str::to_string),
                short_months: [
                    "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月",
                    "12月",
                ]
                .map(str::to_string),
            },
            _ => return Err(ChartError("unknown time locale name".into())),
        };
        locale.validate()?;
        Ok(locale)
    }
}
