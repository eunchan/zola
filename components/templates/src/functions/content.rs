use std::path::PathBuf;
use std::sync::Arc;

use render::RenderCache;
use tera::value::Key;
use tera::{Error, Function, Kwargs, State, TeraResult, Value};

#[derive(Debug)]
pub struct GetPage {
    base_path: PathBuf,
    cache: Arc<RenderCache>,
    default_lang: String,
}

impl GetPage {
    pub fn new(base_path: PathBuf, default_lang: &str, cache: Arc<RenderCache>) -> Self {
        Self { base_path: base_path.join("content"), default_lang: default_lang.to_string(), cache }
    }
}

impl Default for GetPage {
    fn default() -> Self {
        Self {
            base_path: PathBuf::new(),
            default_lang: String::new(),
            cache: Arc::new(RenderCache::default()),
        }
    }
}

impl Function<TeraResult<Value>> for GetPage {
    fn call(&self, kwargs: Kwargs, state: &State) -> TeraResult<Value> {
        let path: String = kwargs.must_get("path")?;
        let lang: String = kwargs
            .get::<String>("lang")?
            .or_else(|| state.get::<String>("lang").ok().flatten())
            .unwrap_or_else(|| self.default_lang.clone());

        let allow_missing: bool = kwargs.get::<bool>("allow_missing")?.unwrap_or(false);

        let full_path = self.base_path.join(&path);

        let res = (|| {
            let cached = self
                .cache
                .pages
                .get(&full_path)
                .ok_or_else(|| Error::message(format!("Page `{}` not found.", path)))?;

            let file_path = self
                .cache
                .pages_by_canonical
                .get(&cached.canonical)
                .and_then(|by_lang| by_lang.get(&lang))
                .ok_or_else(|| {
                    Error::message(format!("Page `{}` not found for language `{}`.", path, lang))
                })?;

            self.cache
                .pages
                .get(file_path)
                .map(|c| c.value.clone())
                .ok_or_else(|| Error::message(format!("Page `{}` not found.", path)))
        })();

        match res {
            Ok(v) => Ok(v),
            Err(err) => {
                if allow_missing {
                    Ok(Value::none())
                } else {
                    Err(err)
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct GetSection {
    base_path: PathBuf,
    cache: Arc<RenderCache>,
    default_lang: String,
}

impl GetSection {
    pub fn new(base_path: PathBuf, default_lang: &str, cache: Arc<RenderCache>) -> Self {
        Self { base_path: base_path.join("content"), default_lang: default_lang.to_string(), cache }
    }
}

impl Default for GetSection {
    fn default() -> Self {
        Self {
            base_path: PathBuf::new(),
            default_lang: String::new(),
            cache: Arc::new(RenderCache::default()),
        }
    }
}

impl Function<TeraResult<Value>> for GetSection {
    fn call(&self, kwargs: Kwargs, state: &State) -> TeraResult<Value> {
        let path: String = kwargs.must_get("path")?;
        let lang: String = kwargs
            .get::<String>("lang")?
            .or_else(|| state.get::<String>("lang").ok().flatten())
            .unwrap_or_else(|| self.default_lang.clone());

        let allow_missing: bool = kwargs.get::<bool>("allow_missing")?.unwrap_or(false);

        let full_path = self.base_path.join(&path);

        let res = (|| {
            let cached = self
                .cache
                .sections
                .get(&full_path)
                .ok_or_else(|| Error::message(format!("Section `{}` not found.", path)))?;

            let file_path = self
                .cache
                .sections_by_canonical
                .get(&cached.canonical)
                .and_then(|by_lang| by_lang.get(&lang))
                .ok_or_else(|| {
                    Error::message(format!("Section `{}` not found for language `{}`.", path, lang))
                })?;

            self.cache
                .sections
                .get(file_path)
                .map(|c| c.value.clone())
                .ok_or_else(|| Error::message(format!("Section `{}` not found.", path)))
        })();

        match res {
            Ok(v) => Ok(v),
            Err(err) => {
                if allow_missing {
                    Ok(Value::none())
                } else {
                    Err(err)
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct GetPages {
    _base_path: PathBuf,
    cache: Arc<RenderCache>,
    default_lang: String,
}

impl GetPages {
    pub fn new(base_path: PathBuf, default_lang: &str, cache: Arc<RenderCache>) -> Self {
        Self { _base_path: base_path.join("content"), default_lang: default_lang.to_string(), cache }
    }
}

impl Default for GetPages {
    fn default() -> Self {
        Self {
            _base_path: PathBuf::new(),
            default_lang: String::new(),
            cache: Arc::new(RenderCache::default()),
        }
    }
}

impl Function<TeraResult<Value>> for GetPages {
    fn call(&self, kwargs: Kwargs, state: &State) -> TeraResult<Value> {
        let lang: String = kwargs
            .get::<String>("lang")?
            .or_else(|| state.get::<String>("lang").ok().flatten())
            .unwrap_or_else(|| self.default_lang.clone());

        let limit: Option<usize> = kwargs.get::<usize>("limit")?;
        let section: Option<String> = kwargs.get::<String>("section")?;
        let sort_by: String = kwargs.get::<String>("sort_by")?.unwrap_or_else(|| "date".to_string());
        let reverse: bool = kwargs.get::<bool>("reverse")?.unwrap_or(false);

        let mut pages: Vec<Value> = Vec::new();

        for by_lang in self.cache.pages_by_canonical.values() {
            if let Some(file_path) = by_lang.get(&lang) {
                if let Some(cached) = self.cache.pages.get(file_path) {
                    let page_val = &cached.value;

                    let is_draft = page_val
                        .as_map()
                        .and_then(|m| m.get(&Key::from("draft")))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    if is_draft {
                        continue;
                    }

                    let is_hidden = page_val
                        .as_map()
                        .and_then(|m| m.get(&Key::from("hidden")))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    if is_hidden {
                        continue;
                    }

                    if let Some(ref sec) = section {
                        let sec_clean = sec
                            .trim_matches('/')
                            .trim_end_matches("/_index.md")
                            .trim_end_matches("_index.md")
                            .trim_matches('/');
                        let sec_parts: Vec<&str> =
                            sec_clean.split('/').filter(|s| !s.is_empty()).collect();

                        let page_components = page_val
                            .as_map()
                            .and_then(|m| m.get(&Key::from("components")))
                            .and_then(|v| v.as_array());

                        let matches_sec = if let Some(components) = page_components {
                            let comp_strs: Vec<&str> =
                                components.iter().filter_map(|c| c.as_str()).collect();
                            comp_strs.starts_with(&sec_parts)
                        } else {
                            false
                        };

                        if !matches_sec {
                            continue;
                        }
                    }

                    pages.push(page_val.clone());
                }
            }
        }

        match sort_by.as_str() {
            "update_date" | "updated" => {
                fn get_effective_date<'a>(val: &'a Value) -> &'a str {
                    let m = match val.as_map() {
                        Some(m) => m,
                        None => return "",
                    };
                    let updated = m.get(&Key::from("updated")).and_then(|v| v.as_str()).unwrap_or("");
                    let date = m.get(&Key::from("date")).and_then(|v| v.as_str()).unwrap_or("");
                    std::cmp::max(updated, date)
                }

                pages.sort_by(|a, b| {
                    let date_a = get_effective_date(a);
                    let date_b = get_effective_date(b);
                    let ord = if reverse {
                        date_a.cmp(date_b)
                    } else {
                        date_b.cmp(date_a)
                    };
                    if ord == std::cmp::Ordering::Equal {
                        let title_a = a.as_map().and_then(|m| m.get(&Key::from("title"))).and_then(|v| v.as_str()).unwrap_or("");
                        let title_b = b.as_map().and_then(|m| m.get(&Key::from("title"))).and_then(|v| v.as_str()).unwrap_or("");
                        title_a.cmp(title_b)
                    } else {
                        ord
                    }
                });
            }
            "date" => {
                pages.sort_by(|a, b| {
                    let date_a = a.as_map().and_then(|m| m.get(&Key::from("date"))).and_then(|v| v.as_str()).unwrap_or("");
                    let date_b = b.as_map().and_then(|m| m.get(&Key::from("date"))).and_then(|v| v.as_str()).unwrap_or("");
                    let ord = if reverse {
                        date_a.cmp(date_b)
                    } else {
                        date_b.cmp(date_a)
                    };
                    if ord == std::cmp::Ordering::Equal {
                        let title_a = a.as_map().and_then(|m| m.get(&Key::from("title"))).and_then(|v| v.as_str()).unwrap_or("");
                        let title_b = b.as_map().and_then(|m| m.get(&Key::from("title"))).and_then(|v| v.as_str()).unwrap_or("");
                        title_a.cmp(title_b)
                    } else {
                        ord
                    }
                });
            }
            "weight" => {
                pages.sort_by(|a, b| {
                    let w_a = a.as_map().and_then(|m| m.get(&Key::from("weight"))).and_then(|v| v.as_i64()).unwrap_or(0);
                    let w_b = b.as_map().and_then(|m| m.get(&Key::from("weight"))).and_then(|v| v.as_i64()).unwrap_or(0);
                    if reverse {
                        w_b.cmp(&w_a)
                    } else {
                        w_a.cmp(&w_b)
                    }
                });
            }
            "title" => {
                pages.sort_by(|a, b| {
                    let title_a = a.as_map().and_then(|m| m.get(&Key::from("title"))).and_then(|v| v.as_str()).unwrap_or("");
                    let title_b = b.as_map().and_then(|m| m.get(&Key::from("title"))).and_then(|v| v.as_str()).unwrap_or("");
                    if reverse {
                        title_b.cmp(title_a)
                    } else {
                        title_a.cmp(title_b)
                    }
                });
            }
            _ => {}
        }

        if let Some(limit) = limit {
            pages.truncate(limit);
        }

        Ok(Value::from(pages))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::Config;
    use content::{FileInfo, Library, Page, Section, SortBy};
    use render::RenderCache;
    use std::path::Path;
    use tera::value::Key;
    use tera::{Context, Kwargs, Tera};

    fn create_page(title: &str, file_path: &str, lang: &str) -> Page {
        let mut page = Page { lang: lang.to_owned(), ..Page::default() };
        page.file = FileInfo::new_page(
            Path::new(format!("/test/base/path/{}", file_path).as_str()),
            &PathBuf::new(),
        );
        page.meta.title = Some(title.to_string());
        page.meta.weight = Some(1);
        page.file.find_language("en", &["fr"]).unwrap();
        page
    }

    fn make_context_with_lang(lang: &str) -> Context {
        let mut ctx = Context::new();
        ctx.insert("lang", &lang);
        ctx
    }

    #[test]
    fn can_get_page() {
        let config = Config::default_for_test();
        let mut library = Library::new(&config);
        let pages = vec![
            ("Homepage", "content/homepage.md", "en"),
            ("Page D'Accueil", "content/homepage.fr.md", "fr"),
            ("Blog", "content/blog.md", "en"),
            ("Wiki", "content/wiki.md", "en"),
            ("Wiki", "content/wiki.fr.md", "fr"),
            ("Recipes", "content/wiki/recipes.md", "en"),
            ("Recettes", "content/wiki/recipes.fr.md", "fr"),
            ("Programming", "content/wiki/programming.md", "en"),
            ("La Programmation", "content/wiki/programming.fr.md", "fr"),
            ("Novels", "content/novels.md", "en"),
            ("Des Romans", "content/novels.fr.md", "fr"),
        ];
        for (t, f, l) in pages.clone() {
            library.insert_page(create_page(t, f, l));
        }
        let tera = Tera::default();
        let mut cache = RenderCache::new(&config);
        cache.build(&library, &[], &tera);
        let base_path = "/test/base/path".into();

        let get_page = GetPage::new(base_path, "en", Arc::new(cache));

        // Find with lang in context
        let kwargs = Kwargs::from([("path", Value::from("wiki/recipes.md"))]);
        let ctx = make_context_with_lang("fr");
        let res = get_page.call(kwargs, &State::new(&ctx)).unwrap();
        let res_obj = res.as_map().unwrap();
        assert_eq!(res_obj.get(&Key::from("title")).unwrap().as_str().unwrap(), "Recettes");

        // Find with lang kwarg (takes precedence over context)
        let kwargs =
            Kwargs::from([("path", Value::from("wiki/recipes.md")), ("lang", Value::from("fr"))]);
        let ctx = Context::new();
        let res = get_page.call(kwargs, &State::new(&ctx)).unwrap();
        let res_obj = res.as_map().unwrap();
        assert_eq!(res_obj.get(&Key::from("title")).unwrap().as_str().unwrap(), "Recettes");

        // Find with default lang (no lang in context)
        let kwargs = Kwargs::from([("path", Value::from("wiki/recipes.md"))]);
        let ctx = Context::new();
        let res = get_page.call(kwargs, &State::new(&ctx)).unwrap();
        let res_obj = res.as_map().unwrap();
        assert_eq!(res_obj.get(&Key::from("title")).unwrap().as_str().unwrap(), "Recipes");

        // Find with default lang when default lang in context
        let kwargs = Kwargs::from([("path", Value::from("wiki/recipes.md"))]);
        let ctx = make_context_with_lang("en");
        let res = get_page.call(kwargs, &State::new(&ctx)).unwrap();
        let res_obj = res.as_map().unwrap();
        assert_eq!(res_obj.get(&Key::from("title")).unwrap().as_str().unwrap(), "Recipes");

        // Error: non-existent path
        let kwargs = Kwargs::from([("path", Value::from("nonexistent.md"))]);
        let ctx = Context::new();
        let res = get_page.call(kwargs, &State::new(&ctx));
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Page `nonexistent.md` not found"));

        // Error: path exists but requested lang translation doesn't
        let kwargs = Kwargs::from([("path", Value::from("blog.md")), ("lang", Value::from("fr"))]);
        let ctx = Context::new();
        let res = get_page.call(kwargs, &State::new(&ctx));
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("not found for language `fr`"));

        // None: non-existent path with allow_missing true
        let kwargs = Kwargs::from([
            ("path", Value::from("nonexistent.md")),
            ("allow_missing", Value::from(true)),
        ]);
        let ctx = Context::new();
        let res = get_page.call(kwargs, &State::new(&ctx));
        assert!(res.is_ok());
        assert!(res.unwrap() == Value::none());
    }

    fn create_section(title: &str, file_path: &str, lang: &str) -> Section {
        let mut section = Section { lang: lang.to_owned(), ..Section::default() };
        section.file = FileInfo::new_section(
            Path::new(format!("/test/base/path/{}", file_path).as_str()),
            &PathBuf::new(),
        );
        section.meta.title = Some(title.to_string());
        section.meta.weight = 1;
        section.meta.transparent = false;
        section.meta.sort_by = SortBy::None;
        section.meta.page_template = Some("new_page.html".to_owned());
        section.file.find_language("en", &["fr"]).unwrap();
        section
    }

    #[test]
    fn can_get_section() {
        let config = Config::default_for_test();
        let mut library = Library::new(&config);
        let sections = vec![
            ("Homepage", "content/_index.md", "en"),
            ("Page D'Accueil", "content/_index.fr.md", "fr"),
            ("Blog", "content/blog/_index.md", "en"),
            ("Wiki", "content/wiki/_index.md", "en"),
            ("Wiki", "content/wiki/_index.fr.md", "fr"),
            ("Recipes", "content/wiki/recipes/_index.md", "en"),
            ("Recettes", "content/wiki/recipes/_index.fr.md", "fr"),
            ("Programming", "content/wiki/programming/_index.md", "en"),
            ("La Programmation", "content/wiki/programming/_index.fr.md", "fr"),
            ("Novels", "content/novels/_index.md", "en"),
            ("Des Romans", "content/novels/_index.fr.md", "fr"),
        ];
        for (t, f, l) in sections.clone() {
            library.insert_section(create_section(t, f, l));
        }
        let tera = Tera::default();
        let mut cache = RenderCache::new(&config);
        cache.build(&library, &[], &tera);
        let base_path = "/test/base/path".into();

        let get_section = GetSection::new(base_path, "en", Arc::new(cache));

        // Find with lang in context
        let kwargs = Kwargs::from([("path", Value::from("wiki/recipes/_index.md"))]);
        let ctx = make_context_with_lang("fr");
        let res = get_section.call(kwargs, &State::new(&ctx)).unwrap();
        let res_obj = res.as_map().unwrap();
        assert_eq!(res_obj.get(&Key::from("title")).unwrap().as_str().unwrap(), "Recettes");

        // Find with lang kwarg (takes precedence over context)
        let kwargs = Kwargs::from([
            ("path", Value::from("wiki/recipes/_index.md")),
            ("lang", Value::from("fr")),
        ]);
        let ctx = Context::new();
        let res = get_section.call(kwargs, &State::new(&ctx)).unwrap();
        let res_obj = res.as_map().unwrap();
        assert_eq!(res_obj.get(&Key::from("title")).unwrap().as_str().unwrap(), "Recettes");

        // Find with default lang (no lang in context)
        let kwargs = Kwargs::from([("path", Value::from("wiki/recipes/_index.md"))]);
        let ctx = Context::new();
        let res = get_section.call(kwargs, &State::new(&ctx)).unwrap();
        let res_obj = res.as_map().unwrap();
        assert_eq!(res_obj.get(&Key::from("title")).unwrap().as_str().unwrap(), "Recipes");

        // Find with default lang when default lang in context
        let kwargs = Kwargs::from([("path", Value::from("wiki/recipes/_index.md"))]);
        let ctx = make_context_with_lang("en");
        let res = get_section.call(kwargs, &State::new(&ctx)).unwrap();
        let res_obj = res.as_map().unwrap();
        assert_eq!(res_obj.get(&Key::from("title")).unwrap().as_str().unwrap(), "Recipes");

        // Error: non-existent path
        let kwargs = Kwargs::from([("path", Value::from("nonexistent/_index.md"))]);
        let ctx = Context::new();
        let res = get_section.call(kwargs, &State::new(&ctx));
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Section `nonexistent/_index.md` not found"));

        // Error: path exists but requested lang translation doesn't
        let kwargs =
            Kwargs::from([("path", Value::from("blog/_index.md")), ("lang", Value::from("fr"))]);
        let ctx = Context::new();
        let res = get_section.call(kwargs, &State::new(&ctx));
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("not found for language `fr`"));

        // Error: non-existent path
        let kwargs = Kwargs::from([
            ("path", Value::from("nonexistent/_index.md")),
            ("allow_missing", Value::from(true)),
        ]);
        let ctx = Context::new();
        let res = get_section.call(kwargs, &State::new(&ctx));
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), Value::none());
    }

    #[test]
    fn can_get_pages() {
        let config = Config::default_for_test();
        let mut library = Library::new(&config);

        let mut p1 = create_page("Post 1", "content/blog/post1.md", "en");
        p1.meta.date = Some("2023-01-01".to_string());
        p1.components = vec!["blog".to_string(), "post1".to_string()];
        library.insert_page(p1);

        let mut p2 = create_page("Post 2", "content/blog/post2.md", "en");
        p2.meta.date = Some("2023-01-05".to_string());
        p2.components = vec!["blog".to_string(), "post2".to_string()];
        library.insert_page(p2);

        let mut p3 = create_page("Wiki Page", "content/wiki/page.md", "en");
        p3.meta.date = Some("2023-01-03".to_string());
        p3.components = vec!["wiki".to_string(), "page".to_string()];
        library.insert_page(p3);

        let tera = Tera::default();
        let mut cache = RenderCache::new(&config);
        cache.build(&library, &[], &tera);
        let base_path: PathBuf = "/test/base/path".into();

        let get_pages = GetPages::new(base_path.clone(), "en", Arc::new(cache));

        // Default sort by date descending
        let kwargs = Kwargs::default();
        let ctx = Context::new();
        let res = get_pages.call(kwargs, &State::new(&ctx)).unwrap();
        let list = res.as_array().unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].as_map().unwrap().get(&Key::from("title")).unwrap().as_str().unwrap(), "Post 2");
        assert_eq!(list[1].as_map().unwrap().get(&Key::from("title")).unwrap().as_str().unwrap(), "Wiki Page");
        assert_eq!(list[2].as_map().unwrap().get(&Key::from("title")).unwrap().as_str().unwrap(), "Post 1");

        // With limit
        let kwargs = Kwargs::from([("limit", Value::from(2))]);
        let res = get_pages.call(kwargs, &State::new(&ctx)).unwrap();
        let list = res.as_array().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].as_map().unwrap().get(&Key::from("title")).unwrap().as_str().unwrap(), "Post 2");
        assert_eq!(list[1].as_map().unwrap().get(&Key::from("title")).unwrap().as_str().unwrap(), "Wiki Page");

        // Filter by section
        let kwargs = Kwargs::from([("section", Value::from("blog"))]);
        let res = get_pages.call(kwargs, &State::new(&ctx)).unwrap();
        let list = res.as_array().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].as_map().unwrap().get(&Key::from("title")).unwrap().as_str().unwrap(), "Post 2");
        assert_eq!(list[1].as_map().unwrap().get(&Key::from("title")).unwrap().as_str().unwrap(), "Post 1");

        // Sort by update_date
        let mut p1_updated = create_page("Post 1 Updated", "content/blog/post1_up.md", "en");
        p1_updated.meta.date = Some("2023-01-01".to_string());
        p1_updated.meta.updated = Some("2023-01-10".to_string());
        p1_updated.components = vec!["blog".to_string(), "post1_up".to_string()];
        library.insert_page(p1_updated);

        let mut cache_up = RenderCache::new(&config);
        cache_up.build(&library, &[], &tera);
        let get_pages_up = GetPages::new(base_path, "en", Arc::new(cache_up));

        let kwargs = Kwargs::from([("sort_by", Value::from("update_date"))]);
        let res = get_pages_up.call(kwargs, &State::new(&ctx)).unwrap();
        let list = res.as_array().unwrap();
        assert_eq!(list[0].as_map().unwrap().get(&Key::from("title")).unwrap().as_str().unwrap(), "Post 1 Updated");
        assert_eq!(list[1].as_map().unwrap().get(&Key::from("title")).unwrap().as_str().unwrap(), "Post 2");
    }
}
