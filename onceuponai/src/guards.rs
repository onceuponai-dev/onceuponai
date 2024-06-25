use actix_session::SessionExt;
use actix_web::guard::Guard;

pub struct AuthGuard;

impl Guard for AuthGuard {
    fn check(&self, ctx: &actix_web::guard::GuardContext<'_>) -> bool {
        let session = ctx.get_session();
        if let Ok(Some(_email)) = session.get::<String>("EMAIL") {
            return true;
        }

        false
    }
}
