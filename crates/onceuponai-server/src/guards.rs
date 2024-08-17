use actix_session::SessionExt;
use actix_web::guard::Guard;

use crate::handlers::auth::verify_pat_token;

#[derive(Clone)]
pub struct AuthGuard {
    pub secret: String,
}

impl Guard for AuthGuard {
    fn check(&self, ctx: &actix_web::guard::GuardContext<'_>) -> bool {
        let session = ctx.get_session();
        if let Ok(Some(_email)) = session.get::<String>("EMAIL") {
            return true;
        }

        let auth_header = ctx.head().headers().get("authorization");
        if let Some(pat_token) = auth_header {
            if let Ok(pat) = pat_token.to_str() {
                let pat = pat.replace("Bearer ", "");
                if verify_pat_token(&pat, &self.secret).is_ok() {
                    return true;
                }
            }
        }

        false
    }
}
