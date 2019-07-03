
use std::env;

use sentry;

pub fn register_sentry() {
    let _sentry = sentry::init(env::var("SENTRY_DSN").unwrap_or_else(|_| "nothing".to_owned()));

    sentry::integrations::panic::register_panic_handler();

    /*
    let credentials = SentryCredentials {
        scheme: env::var("SENTRY_SCHEME").unwrap_or("https".to_owned()),
        key: env::var("SENTRY_KEY").unwrap_or("nothing".to_owned()),
        secret: env::var("SENTRY_SECRET").unwrap_or("nothing".to_owned()),
        host: Some(env::var("SENTRY_HOST").unwrap_or("sentry.io".to_owned())),
        project_id: env::var("SENTRY_PROJECT_ID").unwrap_or("281445".to_owned()),
    };
    let sentry = Sentry::new(
        "waithook".to_string(),
        "0.1.0".to_string(),
        "Production".to_string(),
        credentials
    );

    let default_hook = panic::take_hook();

    sentry.register_panic_handler_with_func(Some(move |panic_info: &PanicInfo| -> () {
        let msg = match panic_info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => {
                match panic_info.payload().downcast_ref::<String>() {
                    Some(s) => &s[..],
                    None => "Box<Any>",
                }
            }
        };
        println!("panic occurred: {}\n  -> {:?}", msg, panic_info);
        default_hook(panic_info);
    }));
    */
}
