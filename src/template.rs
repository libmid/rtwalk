use sailfish::TemplateOnce;

#[derive(TemplateOnce)]
#[template(path = "email_verify.stpl")]
pub struct EmailVerify<'a> {
    pub username: &'a str,
    pub code: u64,
    pub site_name: &'static str,
}
