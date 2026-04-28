# LogSpike Theme Specification

A precise color and typography system for a professional, efficient log viewer. Every value is intentional — derived from macOS system colors, Xcode's editor palette, and the information density requirements of log analysis.

## Design Philosophy

**Utilitarian precision.** LogSpike is a tool for engineers scanning thousands of lines under pressure. Every color choice serves signal clarity — errors must be unmissable, timestamps must be scannable, and the background must disappear. No decorative gradients, no unnecessary depth, no visual noise.

The aesthetic reference points are Xcode's debug console, Tower's commit log, and Proxyman's request inspector — tools that look quiet until something goes wrong, then the problem screams.

## Typography

**Monospaced (log content):** SF Mono — the system monospaced font on macOS. Consistent glyph width is non-negotiable for log alignment.

**UI text:** SF Pro Text (system font) — `.body`, `.caption`, `.headline`. No custom fonts. This is a tool, not a brand.

### Sizing

- Log rows: 12pt SF Mono
- Line numbers: 11pt SF Mono
- Sidebar labels: 13pt SF Pro
- Filter bar labels: 12pt SF Pro
- Status bar: 11pt SF Pro
- Level badges: 10pt SF Pro Medium, uppercased

---

## Dark Theme (Primary)

The default. Optimized for long sessions and low-light environments.

### Surfaces

| Token | Hex | Usage |
|-------|-----|-------|
| windowBackground | #1A1A1E | Main window fill |
| sidebarBackground | #232328 | Sidebar panel |
| tableBackground | #1E1E22 | Log table area |
| filterBarBackground | #232328 | Filter bar strip |
| statusBarBackground | #19191D | Bottom status bar |
| cardBackground | #28282D | Panels, cards |
| inputBackground | #2C2C31 | Text fields, search |

### Borders & Dividers

| Token | Hex | Usage |
|-------|-----|-------|
| border | #3A3A3F | Panel dividers |
| borderSubtle | #2E2E33 | Row separators |
| focusRing | #5BA3FF | Focused input outline (40% opacity) |

### Text

| Token | Hex | Usage |
|-------|-----|-------|
| primaryText | #E5E5EA | Log messages, main content |
| secondaryText | #8E8E93 | Component, sidebar labels |
| tertiaryText | #636366 | Line numbers, disabled |
| timestampText | #7EB6FF | Timestamp column — scannable blue |
| componentText | #A9A9AE | Component/process column |

### Interactive

| Token | Hex | Usage |
|-------|-----|-------|
| sidebarHover | #2C2C31 | Sidebar row hover |
| sidebarActive | #343439 | Selected sidebar row |
| tableRowHover | #FFFFFF at 4% | Log row hover |
| tableRowSelected | #5BA3FF at 15% | Selected log row |
| accentColor | #5BA3FF | Primary accent |

### Log Level Colors (Dark)

| Level | Row Background | Text Color | Badge Background | Badge Text |
|-------|-----------------|--------------|------------------|------------|
| Critical | #FF453A at 16% | #FF6961 | #FF453A | #FFFFFF |
| Error | #FF453A at 11% | #FF6961 | #FF453A at 85% | #FFFFFF |
| Warning | #FFD60A at 9% | #FFD60A | #FFD60A at 80% | #1A1A1E |
| Notice | transparent | #E5E5EA | #5BA3FF at 20% | #7EB6FF |
| Info | transparent | #E5E5EA | #636366 at 28% | #A9A9AE |
| Debug | transparent | #636366 | #636366 at 15% | #636366 |

### Semantic Colors (Dark)

| Token | Hex | Usage |
|-------|-----|-------|
| followingIndicator | #32D74B | Following indicator dot |
| pausedIndicator | #636366 | Paused indicator |
| searchHighlight | #FFD60A at 28% | Search match background |
| searchHighlightActive | #FFD60A at 50% | Current search match |
| errorCodeLink | #5BA3FF | Tappable error codes |
| liveIndicator | #FF9F0A | Live streaming badge |

---

## Light Theme

For well-lit environments. Uses Apple's HIG light system colors.

### Surfaces

| Token | Hex | Usage |
|-------|-----|-------|
| windowBackground | #F5F5F7 | Main window fill |
| sidebarBackground | #EEEFF1 | Sidebar panel |
| tableBackground | #FFFFFF | Log table area |
| filterBarBackground | #EEEFF1 | Filter bar strip |
| statusBarBackground | #E8E8ED | Bottom status bar |
| cardBackground | #FFFFFF | Cards |
| inputBackground | #FFFFFF | Text fields (with border) |

### Borders & Dividers

| Token | Hex | Usage |
|-------|-----|-------|
| border | #D1D1D6 | Panel dividers |
| borderSubtle | #E5E5EA | Row separators |
| focusRing | #007AFF at 40% | Focused input |

### Text

| Token | Hex | Usage |
|-------|-----|-------|
| primaryText | #1D1D1F | Log messages |
| secondaryText | #86868B | Component, sidebar |
| tertiaryText | #AEAEB2 | Line numbers |
| timestampText | #0060CC | Timestamp — dark blue |
| componentText | #6E6E73 | Component column |

### Interactive

| Token | Hex | Usage |
|-------|-----|-------|
| sidebarHover | #E5E5EA | Sidebar hover |
| sidebarActive | #DCDCE0 | Selected sidebar row |
| tableRowHover | #000000 at 3% | Log row hover |
| tableRowSelected | #007AFF at 12% | Selected log row |
| accentColor | #007AFF | Primary accent |

### Log Level Colors (Light)

| Level | Row Background | Text Color | Badge Background | Badge Text |
|-------|-----------------|--------------|------------------|------------|
| Critical | #FF3B30 at 11% | #C5221F | #FF3B30 | #FFFFFF |
| Error | #FF3B30 at 8% | #D32F2F | #FF3B30 at 85% | #FFFFFF |
| Warning | #FF9500 at 9% | #996300 | #FF9500 at 85% | #FFFFFF |
| Notice | transparent | #1D1D1F | #007AFF at 12% | #007AFF |
| Info | transparent | #1D1D1F | #86868B at 15% | #6E6E73 |
| Debug | transparent | #AEAEB2 | #AEAEB2 at 12% | #AEAEB2 |

### Semantic Colors (Light)

| Token | Hex | Usage |
|-------|-----|-------|
| followingIndicator | #34C759 | Following dot |
| pausedIndicator | #AEAEB2 | Paused |
| searchHighlight | #FF9500 at 23% | Search match |
| searchHighlightActive | #FF9500 at 42% | Current match |
| errorCodeLink | #007AFF | Tappable error codes |
| liveIndicator | #FF9500 | Live badge |

---

## High Contrast Theme

For users who want stronger differentiation. Inspired by high-contrast terminal emulators.

### Surfaces

| Token | Hex | Usage |
|-------|-----|-------|
| windowBackground | #0D0D12 | Near-black base |
| sidebarBackground | #12121A | Sidebar |
| tableBackground | #0F0F16 | Log table |
| filterBarBackground | #12121A | Filter bar |
| statusBarBackground | #0A0A10 | Status bar |
| cardBackground | #18182A | Cards |
| inputBackground | #1A1A2E | Inputs |

### Borders

| Token | Hex | Usage |
|-------|-----|-------|
| border | #2A2A40 | Panel dividers |
| borderSubtle | #1E1E30 | Row separators |
| focusRing | #00D9FF at 50% | Focus ring |

### Text

| Token | Hex | Usage |
|-------|-----|-------|
| primaryText | #E0E0F0 | Log text |
| secondaryText | #8080B0 | Supporting text |
| tertiaryText | #5A5A7A | Line numbers |
| timestampText | #00D9FF | Cyan timestamps |
| componentText | #A0A0C8 | Component column |

### Log Level Colors (High Contrast)

| Level | Row Background | Text Color | Badge Background | Badge Text |
|-------|-----------------|--------------|------------------|------------|
| Critical | #FF2050 at 18% | #FF5080 | #FF2050 | #FFFFFF |
| Error | #FF2050 at 13% | #FF5080 | #FF2050 at 85% | #FFFFFF |
| Warning | #FFD000 at 11% | #FFD000 | #FFD000 at 80% | #0D0D12 |
| Notice | transparent | #E0E0F0 | #00D9FF at 20% | #00D9FF |
| Info | transparent | #E0E0F0 | #5A5A7A at 30% | #8080B0 |
| Debug | transparent | #5A5A7A | #5A5A7A at 15% | #5A5A7A |

### Semantic (High Contrast)

| Token | Hex | Usage |
|-------|-----|-------|
| accentColor | #00D9FF | Cyan accent |
| followingIndicator | #00FF88 | Bright green |
| pausedIndicator | #5A5A7A | Dim |
| searchHighlight | #FFD000 at 28% | Yellow search |
| errorCodeLink | #00D9FF | Cyan links |
| liveIndicator | #FF6600 | Orange live |

---

## Component Design Specs

### Level Badge

Dimensions: 18px height, 6px horizontal padding, 4px border-radius
Typography: 10pt SF Pro Medium, uppercased
Layout: Centered in Level column (64px fixed width)

```
 +---------+
 | ERROR   |  ← Red badge, white text
 +---------+
```

### Log Row Layout

Fixed height: 24px (critical for virtual scroll performance)

```
| 4px | Line# | 8px | Timestamp | 8px | Level | 8px | Component | 8px | Message | 4px |
       48px        140px             64px       120px           flex
```

- **Line number:** right-aligned, tertiaryText, 11pt SF Mono
- **Timestamp:** left-aligned, timestampText, 12pt SF Mono
- **Level:** centered badge
- **Component:** left-aligned, truncated, componentText, 12pt SF Mono
- **Message:** left-aligned, primaryText, 12pt SF Mono, fills remaining space
- **Separator:** 1px borderSubtle between rows
- **Error/Warning rows:** full-width background tint
- **Selected row:** tableRowSelected background
- **Hover row:** tableRowHover background

### Filter Bar

Height: 36px

```
| 12px | [search] [field 240px] [.*] [Aa] | 16px | Level chips | flex | Match count | 12px |
```

- Search field: inputBackground, border, 6px radius
- Regex toggle `.*`: 24×24 button, accentColor when active
- Case toggle `Aa`: 24×24 button, accentColor when active
- Level chips: 28px tall, 6px radius, toggleable
- Match count: right-aligned, secondaryText, 11pt SF Pro

### Status Bar

Height: 28px. Bottom of window.

```
| 12px | 45,231 lines | divider | UTF-8 | divider | 2.3 MB | flex | Following/Paused | 12px |
```

- Background: statusBarBackground
- Text: secondaryText, 11pt SF Pro
- Following: 6px circle (followingIndicator) + "Following" text with subtle pulse (2s, opacity 0.6→1.0)
- Paused: gray circle + "Paused" text
- Dividers: 1px vertical borderSubtle

### Sidebar

Width: 220px default (resizable 180–300px)

**Section Headers:**
- 10pt SF Pro Medium, uppercased, tertiaryText, letter-spacing 0.5pt
- 24px height, 12px left padding

**Document Rows:**
- Height: 36px
- Icon: 14px SF Symbol (doc.text or waveform)
- Filename: 13pt SF Pro, primaryText, truncated
- Line count badge: 10pt SF Pro, secondaryText
- Live indicator: 6px dot (liveIndicator)

**Bottom Bar:**
- Settings icon, 36px height, centered
- Top border: borderSubtle

---

## Implementation Notes

- Use CSS custom properties (variables) for all color tokens, organized by theme
- Implement theme switching via `data-theme="dark" | "light" | "high-contrast"`
- All color values support opacity as needed (e.g., `rgba(255, 69, 58, 0.18)`)
- Row heights are fixed (24px for log rows, 36px for sidebar items) to enable efficient virtual scrolling
- Timestamps and error codes are the primary scanning targets — their colors must stand out without being decorative
- Contrast ratios must meet WCAG AA for all text on colored backgrounds
