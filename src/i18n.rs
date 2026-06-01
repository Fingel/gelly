pub use gettextrs::gettext as tr;
pub use gettextrs::ngettext;

pub fn init() {
    gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "");
    gettextrs::bindtextdomain("gelly", env!("LOCALEDIR")).expect("Unable to bind text domain");
    gettextrs::bind_textdomain_codeset("gelly", "UTF-8").expect("Unable to bind codeset");
    gettextrs::textdomain("gelly").expect("Unable to set text domain");
}
