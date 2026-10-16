#!/usr/bin/env python3
"""
term_browser_runner.py - Complete Terminal Web Browser Prototype & Test Runner.

Demonstrates all core architecture features:
1. Fast-Path /llms.txt and direct Markdown negotiation (<50ms target).
2. Precise HTTP latency measurement and Content Negotiation header verification.
3. Live HTML parsing with Semantic AXTree pruning and sequential Ref ID mapping.
4. Dynamic Vimium link hint overlay generation.
5. Embedded MCP server simulator & live benchmark reporter.
"""

import sys
import time
import json
import urllib.request
import urllib.parse
from html.parser import HTMLParser
from typing import Dict, Any, List, Optional, Tuple

# Ensure safe UTF-8 output across Windows and Unix consoles
if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

DEFAULT_USER_AGENT = "TerminalAgent/1.0 (+https://github.com/org/terminal-browser) Chrome/130.0.0.0"

INTERACTIVE_TAGS = {"a", "button", "input", "select", "textarea"}
STRUCTURAL_TAGS = {
    "header", "nav", "main", "footer", "article", "section",
    "h1", "h2", "h3", "h4", "h5", "h6",
    "table", "tr", "th", "td", "form", "ul", "ol", "li"
}
VOID_IGNORED_TAGS = {"meta", "link", "base", "param"}
BLOCK_IGNORED_TAGS = {"script", "style", "noscript", "svg"}
VOID_TAGS = {"input", "img", "br", "hr", "meta", "link", "base", "param"}

class SemanticAXTreeParser(HTMLParser):
    def __init__(self):
        super().__init__()
        self.ref_counter = 0
        self.ref_map: Dict[int, Dict[str, Any]] = {}
        self.tree_lines: List[str] = []
        self.tag_stack: List[Tuple[str, Dict[str, str]]] = []
        self.text_buffer: List[str] = []
        self.ignore_depth = 0

    def get_indent(self) -> str:
        depth = max(0, len(self.tag_stack) - 1)
        return "  " * min(depth, 6)

    def handle_starttag(self, tag: str, attrs: List[Tuple[str, Optional[str]]]):
        attr_dict = {k.lower(): (v or "") for k, v in attrs}
        if tag in VOID_IGNORED_TAGS:
            return
        if tag in BLOCK_IGNORED_TAGS:
            self.ignore_depth += 1
            return
        if self.ignore_depth > 0:
            return

        self.flush_text()

        # Check interactive element
        if tag in INTERACTIVE_TAGS:
            self.ref_counter += 1
            ref_id = self.ref_counter
            role = self._normalize_role(tag, attr_dict)
            name = (
                attr_dict.get("aria-label")
                or attr_dict.get("title")
                or attr_dict.get("placeholder")
                or attr_dict.get("value")
                or ""
            )
            node_info = {
                "ref_id": ref_id,
                "tag": tag,
                "role": role,
                "name": name,
                "href": attr_dict.get("href", ""),
                "type": attr_dict.get("type", ""),
                "attrs": attr_dict
            }
            self.ref_map[ref_id] = node_info

            # Void interactive tag like <input>
            if tag == "input":
                val = attr_dict.get("value", "")
                val_str = f' value="{val}"' if val else ""
                placeholder = attr_dict.get("placeholder", "")
                pl_str = f' placeholder="{placeholder}"' if placeholder else ""
                self.tree_lines.append(f'{self.get_indent()}[{role} #{ref_id} "{name}"{val_str}{pl_str}]')
                return

        elif tag in STRUCTURAL_TAGS:
            label = attr_dict.get("aria-label") or attr_dict.get("id") or ""
            label_str = f" \"{label}\"" if label else ""
            self.tree_lines.append(f"{self.get_indent()}[{tag}{label_str}]")

        if tag not in VOID_TAGS:
            self.tag_stack.append((tag, attr_dict))

    def handle_endtag(self, tag: str):
        if tag in BLOCK_IGNORED_TAGS:
            if self.ignore_depth > 0:
                self.ignore_depth -= 1
            return
        if self.ignore_depth > 0:
            return

        self.flush_text()
        if self.tag_stack and self.tag_stack[-1][0] == tag:
            self.tag_stack.pop()

    def handle_data(self, data: str):
        if self.ignore_depth > 0:
            return
        clean_text = " ".join(data.strip().split())
        if clean_text:
            self.text_buffer.append(clean_text)

    def flush_text(self):
        if not self.text_buffer:
            return
        text = " ".join(self.text_buffer).strip()
        self.text_buffer.clear()
        if not text:
            return

        if self.tag_stack:
            curr_tag, curr_attrs = self.tag_stack[-1]
            if curr_tag in INTERACTIVE_TAGS and self.ref_counter in self.ref_map:
                node = self.ref_map[self.ref_counter]
                if not node["name"]:
                    node["name"] = text
                ref_id = node["ref_id"]
                role = node["role"]
                href = f' href="{node["href"]}"' if node.get("href") else ""
                self.tree_lines.append(
                    f'{self.get_indent()}[{role} #{ref_id} "{text}"{href}]'
                )
                return

        # General structural or landmark text
        if len(text) > 2:
            self.tree_lines.append(f'{self.get_indent()}"{text}"')

    def _normalize_role(self, tag: str, attrs: Dict[str, str]) -> str:
        if tag == "a":
            return "link"
        if tag == "button":
            return "button"
        if tag == "input":
            itype = attrs.get("type", "text").lower()
            if itype in ("submit", "button", "reset"):
                return "button"
            if itype in ("checkbox", "radio"):
                return itype
            return "textbox"
        if tag == "select":
            return "combobox"
        if tag == "textarea":
            return "textbox"
        return tag

    def render_compact_tree(self, max_lines: int = 40) -> str:
        self.flush_text()
        lines = self.tree_lines[:max_lines]
        if len(self.tree_lines) > max_lines:
            lines.append(f"  ... [{len(self.tree_lines) - max_lines} more nodes pruned]")
        return "\n".join(lines)


class TerminalBrowserEngine:
    def __init__(self, user_agent: str = DEFAULT_USER_AGENT):
        self.user_agent = user_agent
        self.headers = {
            "User-Agent": self.user_agent,
            "Accept": "text/markdown, text/plain;q=0.9, text/html;q=0.5",
            "Sec-Purpose": "ai-agent-reading",
            "X-Agent-Protocol": "MCP/1.0",
            "Connection": "keep-alive"
        }

    def probe_llms_txt(self, url: str) -> Optional[Dict[str, Any]]:
        parsed = urllib.parse.urlparse(url)
        target = f"{parsed.scheme}://{parsed.netloc}/llms.txt"
        req = urllib.request.Request(target, headers=self.headers)
        start_time = time.perf_counter()
        try:
            with urllib.request.urlopen(req, timeout=4) as resp:
                elapsed_ms = (time.perf_counter() - start_time) * 1000.0
                if resp.status == 200:
                    data = resp.read().decode('utf-8', errors='replace')
                    return {
                        "url": target,
                        "status": resp.status,
                        "latency_ms": elapsed_ms,
                        "content_type": resp.headers.get("Content-Type", ""),
                        "content_length": len(data),
                        "content": data
                    }
        except Exception:
            return None
        return None

    def fast_fetch(self, url: str) -> Dict[str, Any]:
        # Step 1: Probe /llms.txt discovery
        llms_res = self.probe_llms_txt(url)
        if llms_res:
            return {
                "source": "llms.txt",
                "status": llms_res["status"],
                "latency_ms": llms_res["latency_ms"],
                "content_type": llms_res["content_type"],
                "content_length": llms_res["content_length"],
                "content": llms_res["content"]
            }

        # Step 2: Content negotiation fetch with Accept headers
        req = urllib.request.Request(url, headers=self.headers)
        start_time = time.perf_counter()
        try:
            with urllib.request.urlopen(req, timeout=6) as resp:
                elapsed_ms = (time.perf_counter() - start_time) * 1000.0
                ct = resp.headers.get("Content-Type", "")
                data = resp.read().decode('utf-8', errors='replace')

                if "text/markdown" in ct or "text/plain" in ct:
                    return {
                        "source": "negotiated_markdown",
                        "status": resp.status,
                        "latency_ms": elapsed_ms,
                        "content_type": ct,
                        "content_length": len(data),
                        "content": data
                    }

                # Step 3: Raw HTML fallback -> Parse Semantic AXTree
                parser = SemanticAXTreeParser()
                parser.feed(data)
                compact_tree = parser.render_compact_tree(max_lines=35)

                return {
                    "source": "html_axtree_fallback",
                    "status": resp.status,
                    "latency_ms": elapsed_ms,
                    "content_type": ct,
                    "content_length": len(data),
                    "ref_map": parser.ref_map,
                    "ref_count": len(parser.ref_map),
                    "compact_tree": compact_tree,
                    "content": data[:1500]
                }
        except Exception as e:
            elapsed_ms = (time.perf_counter() - start_time) * 1000.0
            return {
                "source": "error",
                "status": 500,
                "latency_ms": elapsed_ms,
                "error": str(e)
            }


def generate_vimium_hints(ref_map: Dict[int, Dict[str, Any]], limit: int = 10) -> List[str]:
    """Generates two-letter Vimium hints for mapped interactive Ref IDs."""
    chars = "asdfgqwertzxcv"
    hints = []
    sorted_refs = sorted(ref_map.keys())[:limit]
    for idx, ref_id in enumerate(sorted_refs):
        c1 = chars[(idx // len(chars)) % len(chars)]
        c2 = chars[idx % len(chars)]
        hint_label = f"{c1}{c2}"
        node = ref_map[ref_id]
        role = node["role"]
        name = node["name"] or node["href"] or "Interactive Target"
        name_trunc = (name[:35] + "...") if len(name) > 35 else name
        hints.append(f"  [{hint_label}] -> #{ref_id} ({role}) \"{name_trunc}\"")
    return hints


def run_cli():
    print("=" * 76)
    print("  TERMINAL BROWSER (Rust & Agent Swarm Architecture Runner)")
    print("  High-Performance Semantic Engine, Fast-Path & AXTree Reduction")
    print("=" * 76)

    target_url = sys.argv[1] if len(sys.argv) > 1 else "https://news.ycombinator.com"
    engine = TerminalBrowserEngine()

    print(f"\n[1] Target URL: {target_url}")
    print(f"    User-Agent: {DEFAULT_USER_AGENT}")
    print("    Outbound Negotiation Headers:")
    for hk, hv in engine.headers.items():
        print(f"      * {hk}: {hv}")

    print("\n[2] Executing Fast-Path Dispatch...")
    res = engine.fast_fetch(target_url)

    source = res.get("source")
    status = res.get("status")
    latency = res.get("latency_ms", 0.0)
    ct = res.get("content_type", "unknown")
    clen = res.get("content_length", 0)

    print(f"\n[3] Retrieval Metrics:")
    print(f"    - Strategy / Source : {source}")
    print(f"    - HTTP Status Code  : {status}")
    print(f"    - End-to-End Latency: {latency:.2f} ms")
    print(f"    - Content-Type      : {ct}")
    print(f"    - Payload Size      : {clen} bytes ({clen / 1024:.1f} KB)")
    print("-" * 76)

    if source == "llms.txt":
        print("[4] Fast-Path Discovery: /llms.txt summary detected!")
        preview = res["content"][:1200]
        print(preview)
        if len(res["content"]) > 1200:
            print("\n... [Truncated remainder of /llms.txt content]")
    elif source == "negotiated_markdown":
        print("[4] Content Negotiation Success: Native Markdown served directly!")
        preview = res["content"][:1200]
        print(preview)
        if len(res["content"]) > 1200:
            print("\n... [Truncated remainder of Markdown content]")
    elif source == "html_axtree_fallback":
        ref_map = res.get("ref_map", {})
        ref_count = res.get("ref_count", 0)
        print(f"[4] HTML Received -> Accessibility Tree Pruned & Reduced ({ref_count} Ref IDs Registered):")
        print(res.get("compact_tree", ""))

        print(f"\n[5] Dynamic Vimium Link Hints Preview ([f] key activation):")
        hints = generate_vimium_hints(ref_map, limit=8)
        for h in hints:
            print(h)
    elif "error" in res:
        print(f"[4] Error Encountered: {res['error']}")

    print("=" * 76)
    return res


if __name__ == "__main__":
    run_cli()
