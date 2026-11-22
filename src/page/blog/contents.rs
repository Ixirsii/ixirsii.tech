use html5ever::QualName;
use scraper::{ElementRef, Html, Node};

pub(super) struct Heading {
    pub label: String,
    pub id: String,
}

pub(super) struct Section {
    pub heading: Heading,
    pub children: Vec<Heading>,
}

fn heading_label(element: ElementRef<'_>) -> String {
    let mut label = String::new();
    for node in element.descendants() {
        match node.value() {
            Node::Text(text) => label.push_str(text),
            Node::Element(el) if el.name() == "img" => label.push_str(el.attr("alt").unwrap_or("")),
            Node::Element(el) if el.name() == "br" => label.push(' '),
            _ => {}
        }
    }
    label.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Work from the sanitized article so labels match the final text and only
/// generated anchors are added to the page.
pub(super) fn add_heading_anchors(body: &mut String) -> Vec<Section> {
    let mut document = Html::parse_fragment(body);
    let headings: Vec<_> = document
        .root_element()
        .descendants()
        .filter_map(ElementRef::wrap)
        .filter(|el| matches!(el.value().name(), "h2" | "h3"))
        .map(|el| (el.id(), el.value().name() == "h2", heading_label(el)))
        .filter(|(_, _, label)| !label.is_empty())
        .collect();
    let mut contents: Vec<Section> = Vec::new();
    let mut has_parent = false;
    for (index, (node_id, is_section, label)) in headings.into_iter().enumerate() {
        let Some(mut node) = document.tree.get_mut(node_id) else {
            continue;
        };
        let Node::Element(element) = node.value() else {
            continue;
        };
        let id = format!("post-section-{}", index + 1);
        element.attrs.push((
            QualName::new(None, "".into(), "id".into()),
            id.as_str().into(),
        ));
        let heading = Heading { label, id };
        if !is_section
            && has_parent
            && let Some(section) = contents.last_mut()
        {
            section.children.push(heading);
        } else {
            contents.push(Section {
                heading,
                children: Vec::new(),
            });
            has_parent = is_section;
        }
    }
    if !contents.is_empty() {
        *body = document.root_element().inner_html();
    }
    contents
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::render_markdown;

    #[test]
    fn outline_matches_sanitized_headings() {
        let mut body = render_markdown(
            "### Intro\n\n### Preface\n\n## *A* & `B`\n\n### Again\n\n### Again\n\n#### Deep\n\n```\n## Example\n```\n\n<script>bad()</script>",
        );
        let sections = add_heading_anchors(&mut body);
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[2].heading.label, "A & B");
        assert_eq!(sections[2].children.len(), 2);
        assert_ne!(sections[2].children[0].id, sections[2].children[1].id);
        for section in sections {
            for heading in std::iter::once(section.heading).chain(section.children) {
                assert_eq!(body.matches(&format!("id=\"{}\"", heading.id)).count(), 1);
            }
        }
        assert!(!body.contains("bad()"));
    }
}
