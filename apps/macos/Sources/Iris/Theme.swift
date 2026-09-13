import SwiftUI

/// Mirrors `design/tokens.md` exactly — hex values, structure, and the
/// 🟡 Provisional status of the palette itself. If a value here doesn't
/// match `tokens.md`, `tokens.md` is wrong or this file is stale; fix
/// whichever one drifted, don't invent a third value.
enum Palette {
    struct Colors {
        let bgCanvas: Color
        let bgSurface: Color
        let bgSurfaceRaised: Color
        let bgHover: Color
        let bgSelected: Color
        let borderDefault: Color
        let borderStrong: Color
        let textPrimary: Color
        let textSecondary: Color
        let textDisabled: Color
        let textOnAccent: Color
        let accentDefault: Color
        let accentHover: Color
        let accentPressed: Color
        let accentTint: Color
        let success: Color
        let successTint: Color
        let warning: Color
        let warningTint: Color
        let danger: Color
        let dangerTint: Color
        let destructive: Color
        let destructiveTint: Color
    }

    static let light = Colors(
        bgCanvas: Color(hex: 0xFAF7F2),
        bgSurface: Color(hex: 0xFFFDFA),
        bgSurfaceRaised: Color(hex: 0xFFFFFF),
        bgHover: Color(hex: 0xF1ECE3),
        bgSelected: Color(hex: 0xEDEBF7),
        borderDefault: Color(hex: 0xE8E1D6),
        borderStrong: Color(hex: 0xD6CCBC),
        textPrimary: Color(hex: 0x2B2620),
        textSecondary: Color(hex: 0x6B6155),
        textDisabled: Color(hex: 0xB4A99A),
        textOnAccent: Color(hex: 0xFFFFFF),
        accentDefault: Color(hex: 0x5D5AA8),
        accentHover: Color(hex: 0x4E4B94),
        accentPressed: Color(hex: 0x403D7A),
        accentTint: Color(hex: 0xEDEBF7),
        success: Color(hex: 0x4C7A5E),
        successTint: Color(hex: 0xE6EFE9),
        warning: Color(hex: 0x9C7A3C),
        warningTint: Color(hex: 0xF4EEE0),
        danger: Color(hex: 0xA24E42),
        dangerTint: Color(hex: 0xF2E5E2),
        destructive: Color(hex: 0x8B3123),
        destructiveTint: Color(hex: 0xEDD9D4)
    )

    static let dark = Colors(
        bgCanvas: Color(hex: 0x1C1A17),
        bgSurface: Color(hex: 0x242220),
        bgSurfaceRaised: Color(hex: 0x2C2A26),
        bgHover: Color(hex: 0x33302B),
        bgSelected: Color(hex: 0x332F45),
        borderDefault: Color(hex: 0x3A362E),
        borderStrong: Color(hex: 0x4A4539),
        textPrimary: Color(hex: 0xF2EDE4),
        textSecondary: Color(hex: 0xB5AC9C),
        textDisabled: Color(hex: 0x726A5C),
        textOnAccent: Color(hex: 0xFFFFFF),
        accentDefault: Color(hex: 0x8E89DD),
        accentHover: Color(hex: 0xA19CE6),
        accentPressed: Color(hex: 0x7873C4),
        accentTint: Color(hex: 0x332F45),
        success: Color(hex: 0x7FAB8F),
        successTint: Color(hex: 0x2A3830),
        warning: Color(hex: 0xC4A265),
        warningTint: Color(hex: 0x3A331F),
        danger: Color(hex: 0xC77D6E),
        dangerTint: Color(hex: 0x3A2621),
        destructive: Color(hex: 0xD9705A),
        destructiveTint: Color(hex: 0x3D2620)
    )

    static func colors(for scheme: ColorScheme) -> Colors {
        scheme == .dark ? dark : light
    }
}

/// `tokens.md` §2 — 4px base unit. Every padding/gap uses one of these.
enum Space {
    static let xxs: CGFloat = 2
    static let xs: CGFloat = 4
    static let sm: CGFloat = 8
    static let md: CGFloat = 12
    static let lg: CGFloat = 16
    static let xl: CGFloat = 24
    static let xxl: CGFloat = 32
    static let xxxl: CGFloat = 48
}

/// `tokens.md` §4.
enum Radius {
    static let sm: CGFloat = 4
    static let md: CGFloat = 8
    static let lg: CGFloat = 12
    static let pill: CGFloat = 9999
}

/// `tokens.md` §3 — Source Serif 4 for reading content, Public Sans for UI
/// chrome. Falls back to the system serif/sans if the fonts aren't installed
/// rather than silently rendering in the wrong family with no signal.
enum Typography {
    static func serif(_ size: CGFloat, weight: Font.Weight = .regular) -> Font {
        .custom("Source Serif 4", size: size, relativeTo: .body).weight(weight)
    }
    static func sans(_ size: CGFloat, weight: Font.Weight = .regular) -> Font {
        .custom("Public Sans", size: size, relativeTo: .body).weight(weight)
    }
    static func mono(_ size: CGFloat) -> Font {
        .custom("IBM Plex Mono", size: size, relativeTo: .body)
    }

    static func h1() -> Font { serif(24, weight: .semibold) }
    static func bodySans() -> Font { sans(14) }
    static func bodySmall() -> Font { sans(13) }
    static func caption() -> Font { sans(12, weight: .medium) }
}

extension Color {
    init(hex: UInt32) {
        let r = Double((hex >> 16) & 0xFF) / 255
        let g = Double((hex >> 8) & 0xFF) / 255
        let b = Double(hex & 0xFF) / 255
        self.init(red: r, green: g, blue: b)
    }
}

/// Reads the active palette from the environment's color scheme — every
/// screen should use `@Environment(\.colorScheme)` + this, not hardcode
/// `Palette.light`/`.dark` directly.
struct ThemedColors: DynamicProperty {
    @Environment(\.colorScheme) private var colorScheme
    var colors: Palette.Colors { Palette.colors(for: colorScheme) }
}
