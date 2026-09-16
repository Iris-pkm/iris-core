import SwiftUI
import IrisCore

/// Area-domain aggregate over `music-idea` nodes. Audio-waveform metadata is
/// not in the schema yet, so cards deliberately show real shared metadata.
struct MusicIdeasView: View {
    let engine: FfiEngine
    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }
    @State private var ideas: [CachedNode] = []

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: Space.lg) {
                HStack { VStack(alignment: .leading, spacing: Space.xs) { Text("Music Ideas").font(Typography.h1()).foregroundStyle(c.textPrimary); Text("\(ideas.count) captured sketch\(ideas.count == 1 ? "" : "es")").font(Typography.bodySans()).foregroundStyle(c.textSecondary) }; Spacer(); Text("+ New voice memo").font(Typography.bodySans()).foregroundStyle(c.textDisabled) }
                LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())], spacing: Space.md) { ForEach(ideas, id: \.id) { card($0) } }
                if ideas.isEmpty { Text("No music ideas yet.").font(Typography.bodySmall()).foregroundStyle(c.textDisabled).padding(.top, Space.xl) }
            }.padding(EdgeInsets(top: Space.xxl, leading: Space.xxxl, bottom: Space.xl, trailing: Space.xxxl))
        }.background(c.bgCanvas).task { ideas = (try? engine.search(query: "", nodeType: "music-idea", domain: nil, tag: nil)) ?? [] }
    }
    private func card(_ node: CachedNode) -> some View {
        VStack(alignment: .leading, spacing: Space.sm) {
            HStack { Image(systemName: "play.fill").font(.system(size: 10)); Text(title(node)).font(Typography.bodySans()).foregroundStyle(c.textPrimary); Spacer() }
            HStack(alignment: .center, spacing: 3) { ForEach(0..<26, id: \.self) { i in RoundedRectangle(cornerRadius: 2).fill(c.borderStrong).frame(width: 6, height: CGFloat(5 + ((i * 7) % 20))) } }.frame(height: 30)
            HStack(spacing: Space.xs) { tag(node.domain ?? "music"); ForEach(node.tags.split(separator: ",").prefix(2), id: \.self) { tag(String($0)) } }
        }.padding(Space.md).background(c.bgSurface).overlay(RoundedRectangle(cornerRadius: Radius.lg).stroke(c.borderDefault, lineWidth: 1)).clipShape(RoundedRectangle(cornerRadius: Radius.lg))
    }
    private func tag(_ text: String) -> some View { Text(text).font(Typography.caption()).foregroundStyle(c.textSecondary).padding(.horizontal, Space.sm).padding(.vertical, Space.xxs).background(c.bgHover).clipShape(Capsule()) }
    private func title(_ node: CachedNode) -> String { (node.path as NSString).lastPathComponent.replacingOccurrences(of: ".md", with: "") }
}
