# Example Semantic Accessibility Snapshot

This example illustrates the contrast between a raw DOM dump (~15,000 tokens) and an optimized AXTree Snapshot (~180 tokens).

## 1. Raw DOM Snippet (Avoid)
```html
<div class="sc-1f3k91a-0 bYqDqW header-container flex items-center justify-between p-4 bg-white shadow-sm">
  <div class="flex items-center space-x-2">
    <svg class="h-6 w-6 text-blue-600" fill="none" viewBox="0 0 24 24"><path .../></svg>
    <span class="text-xl font-bold tracking-tight text-gray-900">ArXiv Explorer</span>
  </div>
  <div class="relative w-full max-w-md mx-4">
    <input type="text" class="block w-full pl-10 pr-3 py-2 border border-gray-300 rounded-md leading-5 bg-white placeholder-gray-500 focus:outline-none focus:placeholder-gray-400 focus:ring-1 focus:ring-blue-500 focus:border-blue-500 sm:text-sm" placeholder="Search papers by keyword, author, or arxiv ID..." value="">
    <button class="absolute inset-y-0 right-0 px-3 flex items-center bg-blue-600 text-white rounded-r-md hover:bg-blue-700">Search</button>
  </div>
  ... 500 more lines of Tailwind CSS wrapper tags ...
</div>
```

## 2. Pruned Semantic Snapshot (Ideal)
```markdown
<!-- URL: https://arxiv.org/abs/2307.03172 | Title: WebAgent Architecture | Viewport: 1920x1080 (0% Scrolled) -->
[banner]
  [heading:1 "ArXiv Explorer"]
  [textbox #1 "Search papers by keyword, author, or arxiv ID..." value=""]
  [button #2 "Search"]

[navigation]
  - [link #3 "Recent Submissions" href="/list/cs/recent"]
  - [link #4 "Computer Science" href="/archive/cs"]

[main]
  [heading:2 "A Real-World WebAgent with Planning, HTML Pruning, and Action Grounding"]
  [paragraph "Authors: Izzeddin Gur, Hiroki Furuta, Minmin Chen, et al. (Google DeepMind)"]
  [button #5 "Download PDF" href="/pdf/2307.03172"]
  [button #6 "View TeX Source"]
  
  [heading:3 "Abstract"]
  [paragraph "Completing tasks on the web requires grounding natural language instructions into concrete web actions. In this work, we introduce WebAgent, a modular agent architecture that leverages HTML-T5 for dense accessibility tree pruning and action prediction..."]

  [heading:3 "Citations & References"]
  [table 2 cols, 2 rows]
    | Paper Title | Year |
    | [link #7 "Mind2Web: Towards a Generalist Agent for the Web"] | 2023 |
    | [link #8 "WebVoyager: Building an End-to-End Web Agent"] | 2024 |
```

## 3. Normalized Interaction
To download the PDF, the agent issues:
```json
{
  "tool": "browser_act",
  "arguments": {
    "ref_id": 5,
    "action": "click"
  }
}
```
No brittle CSS selectors (`button.sc-1f3k91a-0`) or complex XPath calculations required.
