/// Aggressive CDP resource filter patterns to reduce page weight by 90%+
pub const BLOCKED_PATTERNS: &[&str] = &[
    // Visual & media assets
    "*.png", "*.jpg", "*.jpeg", "*.webp", "*.gif", "*.svg", "*.ico",
    "*.mp4", "*.webm", "*.avi", "*.mp3",
    "*.woff", "*.woff2", "*.ttf", "*.eot", "*.otf",
    
    // Analytics, telemetry, trackers
    "*google-analytics.com*", "*doubleclick.net*", "*googletagmanager.com*",
    "*segment.io*", "*hotjar.com*", "*clarity.ms*", "*amplitude.com*",
    "*facebook.net*", "*tiktok.com*", "*twitter.com/widgets*",
    "*sentry.io*", "*datadoghq.com*",
    
    // Ad networks
    "*adnxs.com*", "*criteo.com*", "*taboola.com*", "*outbrain.com*"
];

/// Headless Chromium CLI launch flags optimized for agent retrieval
pub const CHROMIUM_AGENT_FLAGS: &[&str] = &[
    "--headless=new",
    "--disable-gpu",
    "--blink-settings=imagesEnabled=false",
    "--disable-remote-fonts",
    "--disable-speech-api",
    "--disable-background-networking",
    "--disable-background-timer-throttling",
    "--disable-client-side-phishing-detection",
    "--disable-default-apps",
    "--disable-hang-monitor",
    "--disable-popup-blocking",
    "--disable-prompt-on-repost",
    "--disable-sync",
    "--metrics-recording-only",
    "--no-first-run",
    "--safebrowsing-disable-auto-update",
    "--password-store=basic",
    "--use-mock-keychain",
];
