use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ServerPack {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub server_ids: &'static [&'static str],
}

pub const SERVER_PACKS: &[ServerPack] = &[
    ServerPack {
        id: "dev-core",
        name: "Developer Core",
        description: "Essential local developer toolkit: Local Filesystem, Git, Knowledge Graph Memory, and Web Fetch.",
        server_ids: &["filesystem", "git", "memory", "fetch"],
    },
    ServerPack {
        id: "data",
        name: "Data & Database",
        description: "Local data inspection and SQL operations: PostgreSQL, SQLite, and Persistent Memory.",
        server_ids: &["postgres", "sqlite", "memory"],
    },
    ServerPack {
        id: "web-research",
        name: "Web & Research",
        description: "Internet research, scraping, and browser automation: Brave Search, Web Fetch, and Puppeteer.",
        server_ids: &["brave-search", "fetch", "puppeteer"],
    },
    ServerPack {
        id: "cloud-dev",
        name: "Cloud & Collaboration",
        description: "Cloud containers, version control, GitHub PRs, and Slack messaging.",
        server_ids: &["docker", "git", "github", "slack"],
    },
];

pub fn find_pack(id: &str) -> Option<&'static ServerPack> {
    SERVER_PACKS.iter().find(|p| p.id.eq_ignore_ascii_case(id))
}
