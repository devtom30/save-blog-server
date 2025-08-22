use crate::Conf;
use log::{error, warn};
use regex::Regex;
use scraper::{Html, Selector};
use serde::Deserialize;
use std::{fmt, fs};
use std::fmt::Formatter;
use std::fs::{create_dir_all, exists};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase", tag = "task_type")]
pub enum Task {
    Parse{url: String, body: String, head: String},
    Attach{url: String, file_path: String, page_url: String}
}

impl Task {
    pub(crate) fn url(&self) -> &str {
        match self {
            Task::Parse{url, ..} => url,
            Task::Attach{url, ..} => url
        }
    }
}

/*impl fmt::Display for Task {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Task::Parse{url, body, head} =>
                write!(f, "Task::Parse {} {} {}", url, body, head),
            Task::Attach{url, page_url, file_path} =>
                write!(f, "Task::Attach {} {} {}", url, page_url, file_path)
        }
    }
}*/

pub trait Executable {
    fn execute(&self) -> Result<(Vec<String>, String), String>;
}

impl Executable for Task {
    fn execute(&self) -> Result<(Vec<String>, String), String> {
        let url_last_part = extract_url_last_part(&self.url());
        return match self {
            Task::Parse { url, body, head} => {
                // extract path from URL
                let path = remove_scheme_and_last_path_part_from_url(&self.url());
                if path.is_none() {
                    error!("can't extract path from URL {}", self.url());
                    return Err("can't extract path from URL".to_string());
                }
                let mut path = path.unwrap();
                // and create directory structure
                if create_dir_all(&path).is_err() {
                    error!("path {} can't be created for URL {}", &path, self.url());
                    return Err("path can't be created for URL".to_string());
                }
                
                // save to filesystem
                let mut data = "<html>\n".to_string();
                data.push_str("\n<head>\n");
                data.push_str(head);
                data.push_str("\n</head>\n");
                data.push_str("\n<body>\n");
                data.push_str(body);
                data.push_str("\n</body>\n");
                data.push_str("\n</html>");
                path.push_str("/");
                path.push_str(url_last_part.as_str());
                write_file_at_path(&path, url, &data).unwrap();

                // parse file to extract assets' URL
                let parsed_html = parse_html(data.as_str(), url);

                Ok((parsed_html.assets, url.to_string()))
            },
            Task::Attach { url, file_path, page_url } => {
                let path = remove_scheme_and_last_path_part_from_url(&page_url);
                if path.is_none() {
                    error!("can't extract path from URL {}", self.url());
                    return Err("can't extract path from URL".to_string());
                }
                let mut path = path.unwrap();
                
                let asset_path = remove_scheme_and_last_path_part_from_url(&url);
                if asset_path.is_none() {
                    error!("can't extract path from URL {}", &url);
                    return Err("can't extract path from URL".to_string());
                }
                
                let assets_path = path + "/assets/" + &asset_path.unwrap();
                create_dir_all(&assets_path).unwrap();
                let target_path = assets_path + "/" + &url_last_part;
                warn!("copying {} to {}", file_path, &target_path);
                fs::copy(file_path, target_path).unwrap();
                Ok((vec![], page_url.to_string()))
            }
        };
    }
}

fn write_file_at_path(path: &String, url: &String, data: &String) -> std::io::Result<()> {
    fs::write(path, data)
}

pub enum DESERIALIZATION_ERROR {
    NO_TASK_TYPE,
    MISSING_FIELD,
    UNKNOWN_TASK_TYPE
}

pub struct ParsedHtml {
    assets: Vec<String>,
    url: String
}

/*
extract URLs from these HTML elements:
<iframe />
<object />
<img />
<picture />
<embed />
<object />
<link />
<script />
<audio />
<video />
<track />
<a>
 */
pub fn parse_html(html: &str, url: &str) -> ParsedHtml {
    let document = Html::parse_document(html);
    let mut parsed_html = ParsedHtml {
        url: url.to_string(),
        assets: vec![]
    };

    // a, link
    ["a", "link"].iter().for_each(|element| { 
        let selector = Selector::parse(element).unwrap();
        for element in document.select(&selector) {
            if let Some(url) = element.value().attr("href") {
                if url_to_asset_to_be_downloaded(url)
                    && !parsed_html.assets.contains(&url.to_string()) {
                    parsed_html.assets.push(url.to_string());
                }
            }
        }
    });
    
    // img, iframe, audio, source
    ["img", "iframe", "audio", "source"].iter().for_each(|html_element| {
        let selector = Selector::parse(html_element).unwrap();
        for element in document.select(&selector) {
            if let Some(url) = element.value().attr("src") {
                if url_to_asset_to_be_downloaded(url)
                    && !parsed_html.assets.contains(&url.to_string())
                    && !(html_element.cmp(&"iframe").is_eq() && url.ends_with(".pdf")) {
                    parsed_html.assets.push(url.to_string());
                }
            }
        }
    });

    parsed_html
}

fn url_to_asset_to_be_downloaded(url: &str) -> bool {
    let re_pages = Regex::new(r"^https?://[^/]+/pages/.+").unwrap();
    let re_blog = Regex::new(r"^https?://[^/]+/blog/.+").unwrap();
    
    !url.ends_with(".html")
        && !re_pages.is_match(url)
        && !re_blog.is_match(url)
        && url.ne("https://benvenuti.e-monsite.com/")
        && (
        url.contains("benvenuti")
            || url.contains("bravissimi")
            || url.contains("ekla")
    )
}

pub fn remove_scheme_and_last_path_part_from_url(url: &str) -> Option<String> {
    let parts: Vec<&str> = url.split("/").into_iter().collect::<Vec<&str>>();
    let re = Regex::new(r"^https?://(.+)/([^/]+)$").unwrap();
    if let Some(caps) = re.captures(&url) {
        Some(caps.get(1).unwrap().as_str().to_string())
    } else {
        None
    }
}

pub fn extract_url_last_part(url: &str) -> String {
    let parts: Vec<&str> = url.split("/").into_iter().collect::<Vec<&str>>();
    parts.last().unwrap().to_string()   
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_parse_html_a() {
        let html = r#"
    <ul>
        <a href="link1"></li>
        <a href="link2">Bar</li>
        <a href="link1">Baz</li>
    </ul>
"#;
        let parsed_html = parse_html(html, "url");
        assert_eq!(parsed_html.assets.len(), 2);
        assert_eq!(parsed_html.url, "url".to_string());
        assert_eq!(parsed_html.assets[0], "link1".to_string());
        assert_eq!(parsed_html.assets[1], "link2".to_string());
    }

    #[test]
    fn test_parse_html_a_img() {
        let html = r#"
    <ul>
        <a href="link1"></li>
        <a href="link2">Bar</li>
        <a href="link1">Baz</li>
        <img src="link1">
        <img src="link3">
    </ul>
"#;
        let parsed_html = parse_html(html, "url");
        assert_eq!(parsed_html.assets.len(), 3);
        assert_eq!(parsed_html.url, "url".to_string());
        assert_eq!(parsed_html.assets[0], "link1".to_string());
        assert_eq!(parsed_html.assets[1], "link2".to_string());
        assert_eq!(parsed_html.assets[2], "link3".to_string());
    }

    #[test]
    fn test_remove_scheme_and_last_path_part_from_url() {
        let url = "https://www.example.com/foo/bar/baz";
        let expected = "www.example.com/foo/bar";
        assert_eq!(remove_scheme_and_last_path_part_from_url(url), Some(expected.to_string()));
    }
    
    #[test]
    fn test_deserialize_task_parse() {
        let json = r#"{"task_type": "parse", "url": "https://www.example.com/foo/bar/baz", "body": "body", "head": "head"}"#;
        let task: Task = serde_json::from_str(json).unwrap();
        assert_eq!(task.url(), "https://www.example.com/foo/bar/baz");
        
        if let Task::Parse{url, body, head} = task {
            assert_eq!(body, "body");
            assert_eq!(head, "head");
        } else {
            panic!("task is not Parse");
        }
    }

    #[test]
    fn test_deserialize_task_attach() {
        let json = r#"{"task_type": "attach", "url": "https://www.example.com/foo/bar/baz", "file_path":"/path/to/file", "page_url": "https://www.example.com/foo/bar/baz"}"#;
        let task: Task = serde_json::from_str(json).unwrap();
        assert_eq!(task.url(), "https://www.example.com/foo/bar/baz");

        if let Task::Attach{url, file_path, page_url} = task {
            assert_eq!(file_path, "/path/to/file");
            assert_eq!(page_url, "https://www.example.com/foo/bar/baz");
        } else {
            panic!("task is not Attach");
        }
    }

    #[test]
    fn test_task_parse_execute() {
        let url = String::from("https://uh.com/ma/super/page/page_a_sauver.html");
        let body = String::from("<body>voilà le body</body>");
        let head = String::from("<head>voily le head</head>");
        let task = Task::Parse {
            url: url.clone(),
            body: body.clone(),
            head: head.clone()
        };

        task.execute();

        assert!(exists("uh.com/ma/super/page").is_ok());
        fs::remove_dir_all("uh.com").unwrap();
    }
    
    #[test]
    fn test_extract_url_last_part() {
        let url = String::from("https://uh.com/ma/super/page/page_a_sauver.html");
        assert_eq!(extract_url_last_part(&url), "page_a_sauver.html".to_string());
    }
    
    #[test]
    fn test_extract_url_last_part_no_slash() {
        let url = String::from("https://uh.com/ma/super/page");
        assert_eq!(extract_url_last_part(&url), "page".to_string());
    }
    
    #[test]
    fn test_execute_task_attach() {
        fs::create_dir_all("uh.com/ma/super/page");
        fs::create_dir_all("test/tmp/path/to/file");
        fs::copy("test/resource/asset.txt", "test/tmp/path/to/file/asset.txt").unwrap();
        
        let page_url = String::from("https://uh.com/ma/super/page/page_a_sauver.html");
        let file_path = String::from("test/tmp/path/to/file/asset.txt");
        let url = String::from("https://assets-test.com/mon/super/asset");
        let task = Task::Attach {
            url: url.clone(),
            file_path: file_path.clone(),
            page_url: page_url.clone()
        };
        task.execute();
        
        assert!(exists("uh.com/ma/super/page/assets/assets-test.com/mon/super/asset").is_ok());
        
        fs::remove_dir_all("test/tmp/path").unwrap();
        fs::remove_dir_all("uh.com").unwrap();
    }
}