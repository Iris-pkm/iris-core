import SwiftUI
import IrisCore

/// Global command-palette overlay (screen-flow kind C). It queries the real
/// cache but leaves the underlying `AppShell` route untouched until a result
/// is opened, at which point the overlay dismisses and the shell pushes the
/// selected node into its existing detail route.
struct SearchPaletteView: View {
    let engine: FfiEngine
    let dismiss: () -> Void
    let open: (CachedNode) -> Void

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    private static let filters = ["All", "Project", "Note", "Task", "Area", "Resource"]

    @State private var query = ""
    @State private var filter = "All"
    @State private var results: [CachedNode] = []
    @State private var selectedIndex = 0
    @FocusState private var queryFocused: Bool

    var body: some View {
        ZStack(alignment: .top) {
            c.bgCanvas.opacity(0.80).ignoresSafeArea()

            VStack {
                palette
                    .padding(.top, 56)
                Spacer()
            }
        }
        .onAppear { queryFocused = true }
        .onExitCommand(perform: dismiss)
        .onKeyPress(.upArrow) {
            selectedIndex = max(selectedIndex - 1, 0)
            return .handled
        }
        .onKeyPress(.downArrow) {
            selectedIndex = min(selectedIndex + 1, max(results.count - 1, 0))
            return .handled
        }
        .task(id: "\(query)|\(filter)") { reload() }
    }

    private var palette: some View {
        VStack(spacing: 0) {
            HStack(spacing: Space.md) {
                Image(systemName: "magnifyingglass")
                    .foregroundStyle(c.textSecondary)
                TextField("Search vault…", text: $query)
                    .textFieldStyle(.plain)
                    .font(Typography.sans(15))
                    .foregroundStyle(c.textPrimary)
                    .focused($queryFocused)
                    .onSubmit(openSelection)
                    .onChange(of: query) { _, _ in selectedIndex = 0 }
                Spacer()
            }
            .padding(Space.lg)
            .overlay(alignment: .bottom) { Divider().background(c.borderDefault) }

            HStack(spacing: 6) {
                ForEach(Self.filters, id: \.self) { label in
                    filterChip(label)
                }
            }
            .padding(.horizontal, Space.lg)
            .padding(.vertical, 10)
            .overlay(alignment: .bottom) { Divider().background(c.borderDefault) }

            ScrollView {
                LazyVStack(spacing: Space.xxs) {
                    ForEach(Array(results.enumerated()), id: \.element.id) { index, node in
                        resultRow(node, selected: index == selectedIndex)
                            .contentShape(Rectangle())
                            .onTapGesture { open(node) }
                    }
                }
                .padding(Space.sm)
            }
            .frame(maxHeight: 354)

            HStack(spacing: Space.sm) {
                KeyHint("↑↓")
                Text("navigate")
                KeyHint("↵")
                Text("open")
                KeyHint("esc")
                Text("dismiss")
                Spacer()
                Text("\(results.count) results")
                    .foregroundStyle(c.textDisabled)
            }
            .font(Typography.caption())
            .foregroundStyle(c.textSecondary)
            .padding(.horizontal, Space.lg)
            .padding(.vertical, Space.sm)
            .overlay(alignment: .top) { Divider().background(c.borderDefault) }
        }
        .frame(width: 640)
        .background(c.bgSurfaceRaised)
        .clipShape(RoundedRectangle(cornerRadius: Radius.lg))
        .overlay(RoundedRectangle(cornerRadius: Radius.lg).stroke(c.borderDefault, lineWidth: 1))
        .shadow(color: .black.opacity(colorScheme == .dark ? 0.55 : 0.25), radius: 35, y: 20)
    }

    private func filterChip(_ label: String) -> some View {
        Button {
            filter = label
            selectedIndex = 0
        } label: {
            Text(label)
                .font(Typography.caption())
                .foregroundStyle(filter == label ? c.accentDefault : c.textSecondary)
                .padding(.horizontal, Space.md)
                .padding(.vertical, 5)
                .background(filter == label ? c.bgSelected : c.bgHover)
                .clipShape(Capsule())
                .overlay(Capsule().stroke(filter == label ? c.accentDefault : c.borderDefault, lineWidth: 1))
        }
        .buttonStyle(.plain)
    }

    private func resultRow(_ node: CachedNode, selected: Bool) -> some View {
        HStack(spacing: Space.sm) {
            Image(systemName: symbol(for: node.nodeType))
                .font(.system(size: 14))
                .foregroundStyle(color(for: node.nodeType))
                .frame(width: 16)
            VStack(alignment: .leading, spacing: Space.xxs) {
                Text(title(for: node))
                    .font(Typography.bodySans())
                    .foregroundStyle(c.textPrimary)
                    .lineLimit(1)
                Text(node.path)
                    .font(Typography.mono(11))
                    .foregroundStyle(c.textSecondary)
                    .lineLimit(1)
            }
            Spacer()
            Text(node.nodeType.capitalized)
                .font(Typography.caption())
                .foregroundStyle(c.textDisabled)
        }
        .padding(Space.sm)
        .background(selected ? c.bgHover : Color.clear)
        .clipShape(RoundedRectangle(cornerRadius: Radius.md))
    }

    private func reload() {
        do {
            results = try engine.search(
                query: query,
                nodeType: filter == "All" ? nil : filter.lowercased(),
                domain: nil,
                tag: nil
            )
            selectedIndex = min(selectedIndex, max(results.count - 1, 0))
        } catch {
            results = []
        }
    }

    private func openSelection() {
        guard results.indices.contains(selectedIndex) else { return }
        open(results[selectedIndex])
    }

    private func title(for node: CachedNode) -> String {
        (node.path as NSString).lastPathComponent.replacingOccurrences(of: ".md", with: "")
    }

    private func symbol(for type: String) -> String {
        switch type {
        case "project": "scope"
        case "task": "checkmark.square"
        case "area": "leaf"
        case "resource": "bookmark"
        default: "doc"
        }
    }

    private func color(for type: String) -> Color {
        switch type {
        case "task": c.warning
        case "area": c.success
        default: c.accentDefault
        }
    }
}

private struct KeyHint: View {
    let text: String

    init(_ text: String) { self.text = text }

    var body: some View {
        Text(text)
            .font(.system(size: 11, weight: .medium))
            .padding(.horizontal, 6)
            .padding(.vertical, 1)
            .overlay(RoundedRectangle(cornerRadius: Radius.sm).stroke(Palette.dark.borderDefault, lineWidth: 1))
    }
}
