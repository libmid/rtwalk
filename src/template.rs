use sailfish::TemplateSimple;

#[derive(TemplateSimple)]
#[template(path = "email_verify.html")]
pub struct EmailVerify<'a> {
    pub username: &'a str,
    pub code: u64,
    pub site_name: &'static str,
}
