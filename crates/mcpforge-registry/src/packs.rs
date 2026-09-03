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
        description: "Essential local developer toolkit: Local Filesystem, Git, Knowledge Graph Memory, Web Fetch, and Sequential Thinking.",
        server_ids: &["filesystem", "git", "memory", "fetch", "sequential-thinking"],
    },
    ServerPack {
        id: "data",
        name: "Data & Databases",
        description: "SQL, document, and cache databases: PostgreSQL, MySQL, SQLite, MongoDB, Redis, and Memory.",
        server_ids: &["postgres", "mysql", "sqlite", "mongodb", "redis", "memory"],
    },
    ServerPack {
        id: "web-research",
        name: "Web & Research",
        description: "Internet research, scraping, and browser automation: Brave Search, Tavily AI, Web Fetch, Puppeteer, and Playwright.",
        server_ids: &["brave-search", "tavily", "fetch", "puppeteer", "playwright"],
    },
    ServerPack {
        id: "cloud-dev",
        name: "Cloud & Infrastructure",
        description: "DevOps & cloud monitoring: Docker, Kubernetes, AWS Cloud, Cloudflare, Sentry, and Datadog.",
        server_ids: &["docker", "kubernetes", "aws", "cloudflare", "sentry", "datadog"],
    },
    ServerPack {
        id: "productivity",
        name: "Productivity & Team",
        description: "Task tracking, docs, and team messaging: Linear, Notion, Slack, Discord, Google Drive, and Todoist.",
        server_ids: &["linear", "notion", "slack", "discord", "google-drive", "todoist"],
    },
    ServerPack {
        id: "ai-agent",
        name: "Autonomous Agent Suite",
        description: "Cognitive enhancements for autonomous coding: Persistent Memory, Sequential Thinking, Context7 Docs, and Time.",
        server_ids: &["memory", "sequential-thinking", "context7", "time", "fetch", "filesystem"],
    },
    ServerPack {
        id: "full-stack",
        name: "Full-Stack Web Suite",
        description: "Everything needed for web development: Filesystem, Git, GitHub, PostgreSQL, Redis, Docker, and Web Fetch.",
        server_ids: &["filesystem", "git", "github", "postgres", "redis", "docker", "fetch"],
    },
    ServerPack {
        id: "enterprise",
        name: "Enterprise Workflow",
        description: "Enterprise collaboration and stability: GitHub, GitLab, Jira Software, Slack, Sentry, and Kubernetes.",
        server_ids: &["github", "gitlab", "jira", "slack", "sentry", "kubernetes"],
    },
];

pub fn find_pack(id: &str) -> Option<&'static ServerPack> {
    SERVER_PACKS.iter().find(|p| p.id.eq_ignore_ascii_case(id))
}
