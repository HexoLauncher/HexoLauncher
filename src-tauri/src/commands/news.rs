use serde::Serialize;

use super::modpack_browse::http;
use crate::error::{CmdError, CmdResult};

const FEED: Option<&str> = match option_env!("NOLAUNCHER_NEWS_FEED") {
    Some(url) if !url.is_empty() => Some(url),
    _ => None,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsItem {
    title: String,
    link: Option<String>,
    description: Option<String>,
    pub_date: Option<String>,
}

#[tauri::command]
pub async fn news_latest() -> CmdResult<Option<NewsItem>> {
    let Some(url) = FEED else {
        return Ok(None);
    };
    let body = http()
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| CmdError::launcher_code("news_fetch_failed", &[("detail", &e)]))?
        .text()
        .await
        .map_err(|e| CmdError::launcher_code("news_fetch_failed", &[("detail", &e)]))?;
    Ok(first_item(&body))
}

fn first_item(xml: &str) -> Option<NewsItem> {
    let item = between(xml, "<item", "</item>")?;
    let item = &item[item.find('>')? + 1..];
    let title = tag(item, "title").filter(|t| !t.is_empty())?;
    Some(NewsItem {
        title,
        link: tag(item, "link").filter(|l| l.starts_with("https://") || l.starts_with("http://")),
        description: tag(item, "description").filter(|d| !d.is_empty()),
        pub_date: tag(item, "pubDate"),
    })
}

fn between<'a>(s: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let from = s.find(start)? + start.len();
    let to = from + s[from..].find(end)?;
    Some(&s[from..to])
}

fn tag(item: &str, name: &str) -> Option<String> {
    let open = format!("<{name}");
    let close = format!("</{name}>");
    let mut rest = item;
    let inner = loop {
        let at = rest.find(&open)?;
        let after = &rest[at + open.len()..];
        match after.chars().next() {
            Some('>') | Some(' ') | Some('\t') | Some('\n') | Some('\r') => {
                let gt = after.find('>')?;
                if after[..gt].ends_with('/') {
                    return None;
                }
                let body = &after[gt + 1..];
                break &body[..body.find(&close)?];
            }
            _ => rest = after,
        }
    };
    let inner = inner.trim();
    let text = match inner.strip_prefix("<![CDATA[").and_then(|s| s.strip_suffix("]]>")) {
        Some(raw) => raw.to_string(),
        None => unescape(inner),
    };
    Some(text.trim().to_string())
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let Some(semi) = rest.find(';').filter(|&i| i <= 10) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..semi];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|d| d.parse().ok()))
                .and_then(char::from_u32),
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &rest[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_first_item() {
        let xml = r#"<?xml version="1.0"?><rss><channel><title>Feed</title><item><title>A &amp; B</title><link>https://x.test/a/</link><description><![CDATA[<b>hi</b>]]></description><pubDate>Thu, 26 Nov 2026 00:00:00 GMT</pubDate></item><item><title>Old</title></item></channel></rss>"#;
        let item = first_item(xml).unwrap();
        assert_eq!(item.title, "A & B");
        assert_eq!(item.link.as_deref(), Some("https://x.test/a/"));
        assert_eq!(item.description.as_deref(), Some("<b>hi</b>"));
        assert_eq!(item.pub_date.as_deref(), Some("Thu, 26 Nov 2026 00:00:00 GMT"));
    }

    #[test]
    fn escaped_html_description() {
        let xml = r#"<rss><channel><item><title>T</title><description>&lt;p&gt;Hello &lt;a href="/news/x/"&gt;link&lt;/a&gt;&lt;/p&gt;&#10;&lt;img src="/a.png"&gt;</description></item></channel></rss>"#;
        let item = first_item(xml).unwrap();
        assert_eq!(
            item.description.as_deref(),
            Some("<p>Hello <a href=\"/news/x/\">link</a></p>\n<img src=\"/a.png\">")
        );
    }

    #[test]
    fn empty_feed() {
        assert!(first_item("<rss><channel><title>x</title></channel></rss>").is_none());
    }
}
