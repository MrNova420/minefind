use hickory_resolver::TokioResolver;

fn make_resolver() -> Option<TokioResolver> {
    let (_config, opts) = hickory_resolver::system_conf::read_system_conf().ok()?;
    let resolver = TokioResolver::builder_tokio()
        .ok()?
        .with_options(opts);
    Some(resolver.build())
}

pub async fn resolve_minecraft_srv(domain: &str) -> Vec<(String, u16)> {
    let resolver = match make_resolver() {
        Some(r) => r,
        None => return vec![],
    };
    let srv_name = format!("_minecraft._tcp.{}", domain);
    match resolver.srv_lookup(&srv_name).await {
        Ok(response) => response.iter()
            .map(|srv| (srv.target().to_string().trim_end_matches('.').to_string(), srv.port()))
            .collect(),
        Err(_) => vec![],
    }
}
