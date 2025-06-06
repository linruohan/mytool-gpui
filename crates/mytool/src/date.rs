use gpui::App;
use jiff::{fmt::strtime, tz::TimeZone, Timestamp, ToSpan};

pub fn format_date(date: Timestamp, _cx: &App) -> String {
    let tz = TimeZone::system();
    let zoned = date.to_zoned(tz.clone());
    let zoned_now = Timestamp::now().to_zoned(tz.clone());
    let prefix = if zoned_now.day().eq(&zoned.day()) {
        "Today"
    } else if zoned_now
        .day()
        .eq(&zoned.checked_sub(ToSpan::day(1)).unwrap().day())
    {
        "Yesterday"
    } else {
        "%d. %b %Y"
    };
    let format = format!("{}, %H:%M:%S", prefix);

    strtime::format(format, zoned.datetime()).unwrap()
}
