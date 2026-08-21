from html.parser import HTMLParser
from pathlib import Path
from typing import List, Optional, Set, Tuple


class PageParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.ids: Set[str] = set()
        self.links: List[str] = []
        self.images: List[str] = []
        self.lang_links: List[Tuple[str, str]] = []
        self._language_link: Optional[str] = None

    def handle_starttag(
        self, tag: str, attrs: List[Tuple[str, Optional[str]]]
    ) -> None:
        values = dict(attrs)
        element_id = values.get("id")
        if element_id:
            assert element_id not in self.ids, f"duplicate id: {element_id}"
            self.ids.add(element_id)
        if tag == "a" and values.get("href"):
            href = values["href"]
            assert href is not None
            self.links.append(href)
            if values.get("hreflang"):
                self._language_link = href
        if tag == "img" and values.get("src"):
            src = values["src"]
            assert src is not None
            self.images.append(src)

    def handle_data(self, data: str) -> None:
        if self._language_link and data.strip():
            self.lang_links.append((data.strip(), self._language_link))

    def handle_endtag(self, tag: str) -> None:
        if tag == "a":
            self._language_link = None


def validate_page(path: Path, required_text: str) -> PageParser:
    html = path.read_text()
    assert required_text in html, f"missing localized copy in {path}"
    assert "—" not in html and "–" not in html, f"forbidden dash in {path}"
    parser = PageParser()
    parser.feed(html)
    for href in (link for link in parser.links if link.startswith("#")):
        assert href[1:] in parser.ids, f"missing anchor target {href} in {path}"
    for source in parser.images:
        assert (path.parent / source).exists(), f"missing image {source} for {path}"
    return parser


root = Path(__file__).resolve().parents[1] / "docs"
english = validate_page(root / "index.html", "Know your Codex limit.")
chinese = validate_page(root / "zh-CN" / "index.html", "看清你的 Codex 额度")

assert ("简体中文", "zh-CN/") in english.lang_links
assert ("English", "../") in chinese.lang_links
assert "zh-CN/" in english.links
assert "../" in chinese.links
assert (root / "styles.css").read_text().count("{") == (root / "styles.css").read_text().count("}")

print("GitHub Pages bilingual validation passed")
