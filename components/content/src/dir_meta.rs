use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use config::Config;
use errors::{Context, Result, anyhow};
use tera::value::Key;
use tera::{Map, Value};

use crate::front_matter::PageFrontMatter;
use crate::front_matter::SectionFrontMatter;
use crate::front_matter::toml_value_to_tera;

/// Cascaded directory metadata loaded from `.meta.yml`, `.meta.yaml`, or `.meta.toml`
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DirMeta {
    pub taxonomies: BTreeMap<String, Vec<String>>,
    pub authors: Vec<String>,
    pub template: Option<String>,
    pub page_template: Option<String>,
    pub draft: Option<bool>,
    pub render: Option<bool>,
    pub in_search_index: Option<bool>,
    pub include_in_feeds: Option<bool>,
    pub hidden: Option<bool>,
    pub weight: Option<usize>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub date: Option<String>,
    pub updated: Option<String>,
    pub extra: Map,
}

impl DirMeta {
    /// Merge a child (more specific) DirMeta on top of `self` (parent/ancestor).
    pub fn merge(&mut self, child: DirMeta) {
        // Taxonomies: append terms preserving order, deduplicating
        for (tax, terms) in child.taxonomies {
            let entry = self.taxonomies.entry(tax).or_default();
            for term in terms {
                if !entry.contains(&term) {
                    entry.push(term);
                }
            }
        }

        if !child.authors.is_empty() {
            self.authors = child.authors;
        }

        if child.template.is_some() {
            self.template = child.template;
        }

        if child.page_template.is_some() {
            self.page_template = child.page_template;
        }

        if child.draft.is_some() {
            self.draft = child.draft;
        }

        if child.render.is_some() {
            self.render = child.render;
        }

        if child.in_search_index.is_some() {
            self.in_search_index = child.in_search_index;
        }

        if child.include_in_feeds.is_some() {
            self.include_in_feeds = child.include_in_feeds;
        }

        if child.hidden.is_some() {
            self.hidden = child.hidden;
        }

        if child.weight.is_some() {
            self.weight = child.weight;
        }

        if child.title.is_some() {
            self.title = child.title;
        }

        if child.description.is_some() {
            self.description = child.description;
        }

        if child.date.is_some() {
            self.date = child.date;
        }

        if child.updated.is_some() {
            self.updated = child.updated;
        }

        merge_extra_maps(&mut self.extra, child.extra);
    }

    /// Apply accumulated directory metadata to a page's front matter.
    pub fn apply_to_page(self, page_meta: &mut PageFrontMatter) {
        // 1. Taxonomies:
        // Page's own terms come first; directory terms are appended if not already present.
        for (tax, terms) in self.taxonomies {
            let page_terms = page_meta.taxonomies.entry(tax).or_default();
            for term in terms {
                if !page_terms.contains(&term) {
                    page_terms.push(term);
                }
            }
        }

        // 2. Authors:
        if page_meta.authors.is_empty() && !self.authors.is_empty() {
            page_meta.authors = self.authors;
        }

        // 3. Template:
        if page_meta.template.is_none() && self.template.is_some() {
            page_meta.template = self.template;
        }

        // 4. Draft:
        if let Some(draft) = self.draft {
            if draft {
                page_meta.draft = true;
            }
        }

        // 5. Render:
        if let Some(render) = self.render {
            page_meta.render = render;
        }

        // 6. Hidden:
        if page_meta.hidden.is_none() && self.hidden.is_some() {
            page_meta.hidden = self.hidden;
        }

        // 7. In search index:
        if let Some(in_search) = self.in_search_index {
            page_meta.in_search_index = in_search;
        }

        // 8. Include in feeds:
        if let Some(in_feeds) = self.include_in_feeds {
            page_meta.include_in_feeds = in_feeds;
        }

        // 9. Weight:
        if page_meta.weight.is_none() && self.weight.is_some() {
            page_meta.weight = self.weight;
        }

        // 10. Title:
        if page_meta.title.is_none() && self.title.is_some() {
            page_meta.title = self.title;
        }

        // 11. Description:
        if page_meta.description.is_none() && self.description.is_some() {
            page_meta.description = self.description;
        }

        // 12. Date / Updated:
        if page_meta.date.is_none() && self.date.is_some() {
            page_meta.date = self.date;
            page_meta.date_to_datetime();
        }
        if page_meta.updated.is_none() && self.updated.is_some() {
            page_meta.updated = self.updated;
            page_meta.date_to_datetime();
        }

        // 13. Extra:
        let mut page_extra_map = std::mem::replace(&mut page_meta.extra, Value::none())
            .into_map()
            .unwrap_or_default();
        merge_dir_extra_into_page_extra(&mut page_extra_map, self.extra);
        page_meta.extra = Value::from(page_extra_map);
    }

    /// Apply accumulated directory metadata to a section's front matter.
    pub fn apply_to_section(self, section_meta: &mut SectionFrontMatter) {
        if section_meta.template.is_none() && self.template.is_some() {
            section_meta.template = self.template;
        }

        if section_meta.page_template.is_none() && self.page_template.is_some() {
            section_meta.page_template = self.page_template;
        }

        if let Some(draft) = self.draft {
            if draft {
                section_meta.draft = true;
            }
        }

        if let Some(render) = self.render {
            section_meta.render = render;
        }

        if let Some(in_search) = self.in_search_index {
            section_meta.in_search_index = in_search;
        }

        if let Some(in_feeds) = self.include_in_feeds {
            section_meta.generate_feeds = in_feeds;
        }

        if section_meta.title.is_none() && self.title.is_some() {
            section_meta.title = self.title;
        }

        if section_meta.description.is_none() && self.description.is_some() {
            section_meta.description = self.description;
        }

        let mut section_extra_map = std::mem::replace(&mut section_meta.extra, Value::none())
            .into_map()
            .unwrap_or_default();
        merge_dir_extra_into_page_extra(&mut section_extra_map, self.extra);
        section_meta.extra = Value::from(section_extra_map);
    }
}

/// Recursively merge source extra map into target extra map (source overrides target).
fn merge_extra_maps(target: &mut Map, source: Map) {
    for (k, v) in source {
        if v.is_map() {
            if let Some(target_val) = target.get_mut(&k) {
                if target_val.is_map() {
                    let mut target_sub = std::mem::replace(target_val, Value::none())
                        .into_map()
                        .unwrap_or_default();
                    let source_sub = v.into_map().unwrap_or_default();
                    merge_extra_maps(&mut target_sub, source_sub);
                    *target_val = Value::from(target_sub);
                    continue;
                }
            }
        }
        target.insert(k, v);
    }
}

/// Merge directory extra fields into page extra: page keys take precedence over directory keys.
fn merge_dir_extra_into_page_extra(page_extra: &mut Map, dir_extra: Map) {
    for (k, v) in dir_extra {
        if let Some(page_val) = page_extra.get_mut(&k) {
            if page_val.is_map() && v.is_map() {
                let mut page_sub = std::mem::replace(page_val, Value::none())
                    .into_map()
                    .unwrap_or_default();
                let dir_sub = v.into_map().unwrap_or_default();
                merge_dir_extra_into_page_extra(&mut page_sub, dir_sub);
                *page_val = Value::from(page_sub);
            }
            // Otherwise, page explicitly defined this field, so keep page value!
        } else {
            page_extra.insert(k, v);
        }
    }
}

/// Convert serde_yaml::Value to tera::Value
pub fn yaml_value_to_tera(val: serde_yaml::Value) -> Value {
    match val {
        serde_yaml::Value::Null => Value::none(),
        serde_yaml::Value::Bool(b) => Value::from(b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::from(i)
            } else if let Some(u) = n.as_u64() {
                Value::from(u)
            } else if let Some(f) = n.as_f64() {
                Value::from(f)
            } else {
                Value::none()
            }
        }
        serde_yaml::Value::String(s) => Value::from(s),
        serde_yaml::Value::Sequence(seq) => {
            Value::from(seq.into_iter().map(yaml_value_to_tera).collect::<Vec<_>>())
        }
        serde_yaml::Value::Mapping(m) => {
            let mut map = Map::new();
            for (k, v) in m {
                let key_str = match k {
                    serde_yaml::Value::String(s) => s,
                    serde_yaml::Value::Number(n) => n.to_string(),
                    serde_yaml::Value::Bool(b) => b.to_string(),
                    _ => continue,
                };
                map.insert(Key::from(key_str), yaml_value_to_tera(v));
            }
            Value::from(map)
        }
        serde_yaml::Value::Tagged(tagged) => yaml_value_to_tera(tagged.value),
    }
}

fn extract_yaml_terms(v: serde_yaml::Value) -> Vec<String> {
    match v {
        serde_yaml::Value::Sequence(seq) => seq
            .into_iter()
            .filter_map(|item| match item {
                serde_yaml::Value::String(s) => Some(s),
                serde_yaml::Value::Number(n) => Some(n.to_string()),
                _ => None,
            })
            .collect(),
        serde_yaml::Value::String(s) => vec![s],
        serde_yaml::Value::Number(n) => vec![n.to_string()],
        _ => vec![],
    }
}

fn extract_toml_terms(v: toml::Value) -> Vec<String> {
    match v {
        toml::Value::Array(arr) => arr
            .into_iter()
            .filter_map(|item| match item {
                toml::Value::String(s) => Some(s),
                toml::Value::Integer(i) => Some(i.to_string()),
                _ => None,
            })
            .collect(),
        toml::Value::String(s) => vec![s],
        toml::Value::Integer(i) => vec![i.to_string()],
        _ => vec![],
    }
}

/// Parse a YAML string (from `.meta.yml` or `.meta.yaml`) into a DirMeta.
pub fn parse_yaml_dir_meta(content: &str, config: &Config) -> Result<DirMeta> {
    let val: serde_yaml::Value = match serde_yaml::from_str(content) {
        Ok(v) => v,
        Err(e) => return Err(anyhow!("YAML deserialize error: {:?}", e)),
    };

    let mapping = match val {
        serde_yaml::Value::Mapping(m) => m,
        _ => return Ok(DirMeta::default()),
    };

    let mut dir_meta = DirMeta::default();

    for (k, v) in mapping {
        let key_str = match k.as_str() {
            Some(s) => s,
            None => continue,
        };

        match key_str {
            "taxonomies" => {
                if let serde_yaml::Value::Mapping(tax_map) = v {
                    for (tax_name_val, terms_val) in tax_map {
                        if let Some(tax_name) = tax_name_val.as_str() {
                            let terms = extract_yaml_terms(terms_val);
                            let entry = dir_meta.taxonomies.entry(tax_name.to_string()).or_default();
                            for term in terms {
                                if !entry.contains(&term) {
                                    entry.push(term);
                                }
                            }
                        }
                    }
                }
            }
            "template" => {
                if let Some(s) = v.as_str() {
                    dir_meta.template = Some(s.to_string());
                }
            }
            "page_template" => {
                if let Some(s) = v.as_str() {
                    dir_meta.page_template = Some(s.to_string());
                }
            }
            "authors" => {
                dir_meta.authors = extract_yaml_terms(v);
            }
            "draft" => {
                if let Some(b) = v.as_bool() {
                    dir_meta.draft = Some(b);
                }
            }
            "render" => {
                if let Some(b) = v.as_bool() {
                    dir_meta.render = Some(b);
                }
            }
            "in_search_index" => {
                if let Some(b) = v.as_bool() {
                    dir_meta.in_search_index = Some(b);
                }
            }
            "include_in_feeds" => {
                if let Some(b) = v.as_bool() {
                    dir_meta.include_in_feeds = Some(b);
                }
            }
            "hidden" => {
                if let Some(b) = v.as_bool() {
                    dir_meta.hidden = Some(b);
                }
            }
            "weight" => {
                if let Some(w) = v.as_u64() {
                    dir_meta.weight = Some(w as usize);
                }
            }
            "title" => {
                if let Some(s) = v.as_str() {
                    dir_meta.title = Some(s.to_string());
                }
            }
            "description" => {
                if let Some(s) = v.as_str() {
                    dir_meta.description = Some(s.to_string());
                }
            }
            "date" => {
                if let Some(s) = v.as_str() {
                    dir_meta.date = Some(s.to_string());
                }
            }
            "updated" => {
                if let Some(s) = v.as_str() {
                    dir_meta.updated = Some(s.to_string());
                }
            }
            "extra" => {
                if let serde_yaml::Value::Mapping(extra_map) = v {
                    for (extra_k, extra_v) in extra_map {
                        if let Some(k_str) = extra_k.as_str() {
                            dir_meta.extra.insert(Key::from(k_str.to_string()), yaml_value_to_tera(extra_v));
                        }
                    }
                }
            }
            other => {
                let is_taxonomy = other == "categories"
                    || other == "tags"
                    || config.taxonomies.iter().any(|t| t.name == other);

                if is_taxonomy {
                    let terms = extract_yaml_terms(v);
                    let entry = dir_meta.taxonomies.entry(other.to_string()).or_default();
                    for term in terms {
                        if !entry.contains(&term) {
                            entry.push(term);
                        }
                    }
                } else {
                    dir_meta.extra.insert(Key::from(other.to_string()), yaml_value_to_tera(v));
                }
            }
        }
    }

    Ok(dir_meta)
}

/// Parse a TOML string (from `.meta.toml`) into a DirMeta.
pub fn parse_toml_dir_meta(content: &str, config: &Config) -> Result<DirMeta> {
    let val: toml::Value = match toml::from_str(content) {
        Ok(v) => v,
        Err(e) => return Err(anyhow!("TOML deserialize error: {:?}", e)),
    };

    let table = match val {
        toml::Value::Table(t) => t,
        _ => return Ok(DirMeta::default()),
    };

    let mut dir_meta = DirMeta::default();

    for (k, v) in table {
        match k.as_str() {
            "taxonomies" => {
                if let toml::Value::Table(tax_table) = v {
                    for (tax_name, terms_val) in tax_table {
                        let terms = extract_toml_terms(terms_val);
                        let entry = dir_meta.taxonomies.entry(tax_name).or_default();
                        for term in terms {
                            if !entry.contains(&term) {
                                entry.push(term);
                            }
                        }
                    }
                }
            }
            "template" => {
                if let toml::Value::String(s) = v {
                    dir_meta.template = Some(s);
                }
            }
            "page_template" => {
                if let toml::Value::String(s) = v {
                    dir_meta.page_template = Some(s);
                }
            }
            "authors" => {
                dir_meta.authors = extract_toml_terms(v);
            }
            "draft" => {
                if let toml::Value::Boolean(b) = v {
                    dir_meta.draft = Some(b);
                }
            }
            "render" => {
                if let toml::Value::Boolean(b) = v {
                    dir_meta.render = Some(b);
                }
            }
            "in_search_index" => {
                if let toml::Value::Boolean(b) = v {
                    dir_meta.in_search_index = Some(b);
                }
            }
            "include_in_feeds" => {
                if let toml::Value::Boolean(b) = v {
                    dir_meta.include_in_feeds = Some(b);
                }
            }
            "hidden" => {
                if let toml::Value::Boolean(b) = v {
                    dir_meta.hidden = Some(b);
                }
            }
            "weight" => {
                if let toml::Value::Integer(w) = v {
                    dir_meta.weight = Some(w as usize);
                }
            }
            "title" => {
                if let toml::Value::String(s) = v {
                    dir_meta.title = Some(s);
                }
            }
            "description" => {
                if let toml::Value::String(s) = v {
                    dir_meta.description = Some(s);
                }
            }
            "date" => {
                if let toml::Value::String(s) = v {
                    dir_meta.date = Some(s);
                } else if let toml::Value::Datetime(d) = v {
                    dir_meta.date = Some(d.to_string());
                }
            }
            "updated" => {
                if let toml::Value::String(s) = v {
                    dir_meta.updated = Some(s);
                } else if let toml::Value::Datetime(d) = v {
                    dir_meta.updated = Some(d.to_string());
                }
            }
            "extra" => {
                if let toml::Value::Table(extra_table) = v {
                    for (extra_k, extra_v) in extra_table {
                        dir_meta.extra.insert(Key::from(extra_k), toml_value_to_tera(extra_v));
                    }
                }
            }
            other => {
                let is_taxonomy = other == "categories"
                    || other == "tags"
                    || config.taxonomies.iter().any(|t| t.name == other);

                if is_taxonomy {
                    let terms = extract_toml_terms(v);
                    let entry = dir_meta.taxonomies.entry(other.to_string()).or_default();
                    for term in terms {
                        if !entry.contains(&term) {
                            entry.push(term);
                        }
                    }
                } else {
                    dir_meta.extra.insert(Key::from(other.to_string()), toml_value_to_tera(v));
                }
            }
        }
    }

    Ok(dir_meta)
}

/// Look for `.meta.yml`, `.meta.yaml`, or `.meta.toml` in `dir` and load it if present.
pub fn load_dir_meta_for_dir(dir: &Path, config: &Config) -> Result<Option<DirMeta>> {
    let candidates = [
        (".meta.yml", true),
        (".meta.yaml", true),
        (".meta.toml", false),
    ];

    for (filename, is_yaml) in candidates {
        let meta_file = dir.join(filename);
        if meta_file.is_file() {
            let content = utils::fs::read_file(&meta_file)
                .with_context(|| format!("Failed to read metadata file `{}`", meta_file.display()))?;
            let meta = if is_yaml {
                parse_yaml_dir_meta(&content, config)
            } else {
                parse_toml_dir_meta(&content, config)
            }
            .with_context(|| format!("Failed to parse metadata file `{}`", meta_file.display()))?;

            return Ok(Some(meta));
        }
    }

    Ok(None)
}

/// Collect all directories from `content/` down to `file_path.parent()` in root-to-leaf order.
pub fn get_parent_dirs_in_content(file_path: &Path, base_path: &Path) -> Vec<PathBuf> {
    let content_dir = if base_path.as_os_str().is_empty() {
        PathBuf::from("content")
    } else {
        base_path.join("content")
    };

    let rel_path = if let Ok(rel) = file_path.strip_prefix(&content_dir) {
        Some(rel.to_path_buf())
    } else if let (Ok(canon_file), Ok(canon_content)) = (file_path.canonicalize(), content_dir.canonicalize()) {
        canon_file.strip_prefix(&canon_content).ok().map(|p| p.to_path_buf())
    } else {
        None
    };

    if let Some(rel) = rel_path {
        let mut dirs = vec![content_dir.clone()];
        let mut cur = content_dir;
        if let Some(parent) = rel.parent() {
            for component in parent.components() {
                cur = cur.join(component.as_os_str());
                dirs.push(cur.clone());
            }
        }
        dirs
    } else {
        // Fallback when not under base_path/content (e.g. unit tests or flat directories)
        let mut dirs = Vec::new();
        let mut cur = file_path.parent();
        while let Some(dir) = cur {
            if dir.as_os_str().is_empty() {
                break;
            }
            dirs.push(dir.to_path_buf());
            if dir == base_path || dir.file_name().map_or(false, |n| n == "content") {
                break;
            }
            cur = dir.parent();
        }
        dirs.reverse();
        dirs
    }
}

/// Accumulate cascaded DirMeta from all ancestor directories down to `file_path.parent()`.
pub fn get_cascaded_dir_meta(
    file_path: &Path,
    base_path: &Path,
    config: &Config,
) -> Result<DirMeta> {
    let dirs = get_parent_dirs_in_content(file_path, base_path);
    let mut accumulated = DirMeta::default();

    for dir in dirs {
        if let Some(child_meta) = load_dir_meta_for_dir(&dir, config)? {
            accumulated.merge(child_meta);
        }
    }

    Ok(accumulated)
}

/// Apply cascaded directory metadata to a PageFrontMatter.
pub fn apply_dir_meta_to_page(
    file_path: &Path,
    base_path: &Path,
    config: &Config,
    meta: &mut PageFrontMatter,
) -> Result<()> {
    let dir_meta = get_cascaded_dir_meta(file_path, base_path, config)?;
    dir_meta.apply_to_page(meta);
    Ok(())
}

/// Apply cascaded directory metadata to a SectionFrontMatter.
pub fn apply_dir_meta_to_section(
    file_path: &Path,
    base_path: &Path,
    config: &Config,
    meta: &mut SectionFrontMatter,
) -> Result<()> {
    let dir_meta = get_cascaded_dir_meta(file_path, base_path, config)?;
    dir_meta.apply_to_section(meta);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_parse_yaml_taxonomies_and_extra() {
        let config = Config::default();
        let yaml = r#"
categories: ['design', 'hardware']
toc: true
comments: false
authors:
  - Eli Kim
template: "custom.html"
"#;
        let meta = parse_yaml_dir_meta(yaml, &config).unwrap();
        assert_eq!(meta.taxonomies.get("categories").unwrap(), &vec!["design", "hardware"]);
        assert_eq!(meta.authors, vec!["Eli Kim"]);
        assert_eq!(meta.template, Some("custom.html".to_string()));
        assert_eq!(meta.extra.get(&Key::from("toc")).unwrap(), &Value::from(true));
        assert_eq!(meta.extra.get(&Key::from("comments")).unwrap(), &Value::from(false));
    }

    #[test]
    fn test_parse_yaml_taxonomies_block() {
        let config = Config::default();
        let yaml = r#"
taxonomies:
  categories:
    - design
  tags:
    - fpga
    - verilog
extra:
  toc: true
"#;
        let meta = parse_yaml_dir_meta(yaml, &config).unwrap();
        assert_eq!(meta.taxonomies.get("categories").unwrap(), &vec!["design"]);
        assert_eq!(meta.taxonomies.get("tags").unwrap(), &vec!["fpga", "verilog"]);
        assert_eq!(meta.extra.get(&Key::from("toc")).unwrap(), &Value::from(true));
    }

    #[test]
    fn test_parse_toml_dir_meta() {
        let config = Config::default();
        let toml_str = r#"
categories = ["design"]
authors = ["Eli Kim"]
template = "blog.html"

[extra]
toc = true
"#;
        let meta = parse_toml_dir_meta(toml_str, &config).unwrap();
        assert_eq!(meta.taxonomies.get("categories").unwrap(), &vec!["design"]);
        assert_eq!(meta.authors, vec!["Eli Kim"]);
        assert_eq!(meta.template, Some("blog.html".to_string()));
        assert_eq!(meta.extra.get(&Key::from("toc")).unwrap(), &Value::from(true));
    }

    #[test]
    fn test_cascading_merge_and_apply_to_page() {
        let tmp = tempdir().unwrap();
        let base_path = tmp.path();
        let content_dir = base_path.join("content");
        let parent_dir = content_dir.join("blog").join("posts");
        let child_dir = parent_dir.join("design");
        fs::create_dir_all(&child_dir).unwrap();

        // Write parent .meta.yml
        fs::write(
            parent_dir.join(".meta.yml"),
            "authors: ['Parent Author']\ntags: ['parent_tag']\ntoc: false\n",
        )
        .unwrap();

        // Write child .meta.yaml
        fs::write(
            child_dir.join(".meta.yaml"),
            "categories: ['child_cat']\ntags: ['child_tag']\ntoc: true\n",
        )
        .unwrap();

        let page_file = child_dir.join("post.md");
        let config = Config::default();

        let cascaded = get_cascaded_dir_meta(&page_file, base_path, &config).unwrap();
        assert_eq!(cascaded.authors, vec!["Parent Author"]);
        assert_eq!(cascaded.taxonomies.get("categories").unwrap(), &vec!["child_cat"]);
        assert_eq!(cascaded.taxonomies.get("tags").unwrap(), &vec!["parent_tag", "child_tag"]);
        // Child's toc: true overrode parent's toc: false
        assert_eq!(cascaded.extra.get(&Key::from("toc")).unwrap(), &Value::from(true));

        // Now apply to page with its own tags and toc = false
        let mut page_meta = PageFrontMatter::default();
        page_meta.taxonomies.insert("tags".to_string(), vec!["page_tag".to_string()]);
        let mut initial_extra = Map::new();
        initial_extra.insert(Key::from("toc"), Value::from(false));
        page_meta.extra = Value::from(initial_extra);

        cascaded.apply_to_page(&mut page_meta);

        // Page tags should come first, then inherited tags appended
        assert_eq!(
            page_meta.taxonomies.get("tags").unwrap(),
            &vec!["page_tag", "parent_tag", "child_tag"]
        );
        assert_eq!(page_meta.taxonomies.get("categories").unwrap(), &vec!["child_cat"]);
        assert_eq!(page_meta.authors, vec!["Parent Author"]);
        // Page's own toc: false should NOT be overridden by directory's toc: true!
        assert_eq!(
            page_meta.extra.as_map().unwrap().get(&Key::from("toc")).unwrap(),
            &Value::from(false)
        );
    }
}
