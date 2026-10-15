#!/usr/bin/env python3
"""
extract_axtree.py - Standalone Chrome DevTools Protocol (CDP) Accessibility Tree Pruner.

Connects to a running Chromium instance with --remote-debugging-port=9222,
fetches the Accessibility Tree (AXTree), filters non-interactive noise,
assigns sequential numeric Ref IDs, and prints a compact token-efficient markdown representation.
"""

import sys
import json
import urllib.request
import urllib.error

INTERACTIVE_ROLES = {
    "button", "link", "textbox", "combobox", "checkbox", "radio",
    "switch", "slider", "menuitem", "tab", "searchbox"
}

STRUCTURAL_ROLES = {
    "banner", "navigation", "main", "contentinfo", "heading",
    "table", "row", "cell", "list", "listitem", "dialog", "alert"
}

def get_websocket_debugger_url(port=9222):
    try:
        url = f"http://127.0.0.1:{port}/json"
        with urllib.request.urlopen(url, timeout=3) as resp:
            data = json.loads(resp.read().decode('utf-8'))
            for page in data:
                if page.get("type") == "page":
                    return page.get("webSocketDebuggerUrl")
    except Exception as e:
        sys.stderr.write(f"Failed to connect to Chromium on port {port}: {e}\n")
    return None

def prune_and_format_ax_node(node, ref_counter, depth=0):
    role = node.get("role", {}).get("value", "")
    name = node.get("name", {}).get("value", "").strip()
    value = node.get("value", {}).get("value", "").strip()
    
    lines = []
    ref_id = None
    is_interactive = role in INTERACTIVE_ROLES
    
    if is_interactive:
        ref_counter[0] += 1
        ref_id = ref_counter[0]
        label = f'"{name}"' if name else ""
        val_str = f' value="{value}"' if value else ""
        lines.append(f"{'  ' * depth}[{role} #{ref_id} {label}{val_str}]".strip())
    elif role in STRUCTURAL_ROLES:
        label = f' "{name}"' if name else ""
        lines.append(f"{'  ' * depth}[{role}{label}]")
    elif name and len(name) > 1 and role not in ("generic", "none"):
        lines.append(f"{'  ' * depth}[{role} \"{name}\"]")

    children = node.get("children", [])
    for child in children:
        child_lines = prune_and_format_ax_node(child, ref_counter, depth + (1 if lines else 0))
        lines.extend(child_lines)

    return lines

def format_axtree_snapshot(raw_tree):
    ref_counter = [0]
    output_lines = [
        "<!-- Snapshot Generated via Semantic AXTree Pruner -->"
    ]
    for root_node in raw_tree.get("nodes", []):
        output_lines.extend(prune_and_format_ax_node(root_node, ref_counter))
    return "\n".join(output_lines)

if __name__ == "__main__":
    print("Semantic AXTree Extraction Protocol ready.")
    print("To use: launch chromium with --remote-debugging-port=9222 and connect.")
