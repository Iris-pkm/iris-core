import SwiftUI
import IrisCore

/// The persistent three-pane shell (`design/navigation.md` §1): sidebar +
/// main content pane + optional right panel. Mounts once a vault is open
/// (`design/screen-flow.md`'s "Onboarding -> app shell" edge, kind A-like
/// shell-in transition) and stays for the rest of the session.
struct AppShell: View {
    let engine: FfiEngine
    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    /// The currently open node, or `nil` if nothing's open yet. Real entry
    /// points (clicking a row in a list view) don't exist yet — task-view
    /// lenses and the PARA workbench are next in this builder's queue
    /// (`design/screen-flow.md` §5) — so this screen is reachable via the
    /// sidebar's real Project/Area/Resource items for now. Keeping the whole
    /// `CachedNode` (not just its path) so the shell can route projects to
    /// `ProjectView` vs. everything else to the generic `NodeEditorView`
    /// without a second lookup.
    @State private var openNode: CachedNode?
    /// The five task-view lenses (`design/navigation.md`'s Inbox/Today/
    /// Upcoming/Someday-Maybe/Logbook rows) are sibling sidebar
    /// destinations, not nodes — a separate selection from `openNode`.
    /// Picking a lens clears `openNode` and vice versa; only one of the
    /// two ever drives the main content pane at a time.
    @State private var selectedLens: TaskLens?

    var body: some View {
        HStack(spacing: 0) {
            Sidebar(engine: engine, openNode: $openNode, selectedLens: $selectedLens)
            Divider().background(c.borderDefault)

            if let selectedLens {
                lensView(for: selectedLens)
            } else if let openNode {
                detail(for: openNode)
            } else {
                emptyState
            }
        }
        .background(c.bgCanvas)
        .frame(minWidth: 900, idealWidth: 1280, minHeight: 640, idealHeight: 800)
    }

    /// A task row inside any lens opens straight into the Node Editor
    /// (`navigation.md` §3: "clicking a row in any list view transitions
    /// ... -> Node Editor") — clears the lens selection so `openNode`
    /// drives the content pane, matching how a Project/Area/Resource
    /// sidebar click already behaves.
    @ViewBuilder
    private func lensView(for lens: TaskLens) -> some View {
        let onOpen: (CachedNode) -> Void = { node in
            selectedLens = nil
            openNode = node
        }
        switch lens {
        case .inbox: InboxView(engine: engine, onOpenNode: onOpen)
        case .today: TodayView(engine: engine, onOpenNode: onOpen)
        case .upcoming: UpcomingView(engine: engine, onOpenNode: onOpen)
        case .somedayMaybe: SomedayMaybeView(engine: engine, onOpenNode: onOpen)
        case .logbook: LogbookView(engine: engine, onOpenNode: onOpen)
        }
    }

    /// Projects get `ProjectView` full-width, no right rail — the mockup
    /// (`Main.dc.html` vs. `GuidedActivation.dc.html`) confirms Guided
    /// Activation and the normal project layout both replace the *entire*
    /// content pane, Connections/Dev Mode included, not just the main
    /// column next to an unchanged rail. Every other node type keeps the
    /// generic Node Editor + Connections/Dev Mode rail.
    @ViewBuilder
    private func detail(for node: CachedNode) -> some View {
        if node.nodeType == "project" {
            ProjectView(engine: engine, relPath: node.path)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else {
            HStack(spacing: 0) {
                NodeEditorView(engine: engine, relPath: node.path)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                Divider().background(c.borderDefault)
                RightRail(engine: engine, relPath: node.path)
            }
        }
    }

    private var emptyState: some View {
        VStack(spacing: Space.sm) {
            Text("No node open").font(Typography.h1()).foregroundStyle(c.textPrimary)
            Text("Pick a project, area, or resource from the sidebar.")
                .font(Typography.bodySans())
                .foregroundStyle(c.textSecondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

/// Real Project/Area/Resource nodes via `FfiEngine.search`'s `nodeType`
/// filter — not the PARA workbench screen itself (list/table/kanban modes,
/// `design/navigation.md`'s PARA workbench row), just enough real sidebar
/// content for this builder's Node Editor work. Clicking an item opens it
/// directly in the Node Editor.
///
/// **Flag for `design/navigation.md`/`screen-flow.md`:** the mockup
/// (`Main.dc.html`) shows sidebar Project/Area/Resource entries as
/// individual named nodes that open straight into the Node Editor on
/// click — not, as `screen-flow.md`'s edge table currently groups them, a
/// kind-A sibling swap into a shared "PARA workbench sections" screen.
/// Worth reconciling once the PARA workbench is actually built.
private struct Sidebar: View {
    let engine: FfiEngine
    @Binding var openNode: CachedNode?
    @Binding var selectedLens: TaskLens?
    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    @State private var projects: [CachedNode] = []
    @State private var areas: [CachedNode] = []
    @State private var resources: [CachedNode] = []
    @State private var lensCounts: [TaskLens: Int] = [:]

    var body: some View {
        VStack(alignment: .leading, spacing: Space.lg) {
            HStack(spacing: Space.sm) {
                logo
                Text("Iris").font(Typography.serif(15, weight: .semibold)).foregroundStyle(c.textPrimary)
            }

            searchBar

            VStack(alignment: .leading, spacing: 1) {
                ForEach(TaskLens.allCases) { lens in
                    lensRow(lens)
                }
            }

            ScrollView {
                VStack(alignment: .leading, spacing: Space.lg) {
                    section(title: "Projects", dotColor: c.warning, nodes: projects)
                    section(title: "Areas", dotColor: c.success, nodes: areas)
                    section(title: "Resources", dotColor: c.accentDefault, nodes: resources)
                }
            }

            Spacer()
            graphButton
        }
        .padding(Space.md)
        .frame(width: 236)
        .background(c.bgSurface)
        .task { reload() }
    }

    private func lensRow(_ lens: TaskLens) -> some View {
        let isActive = selectedLens == lens
        return Button {
            openNode = nil
            selectedLens = lens
        } label: {
            HStack {
                Text(lens.title).font(Typography.bodySans()).foregroundStyle(c.textPrimary)
                Spacer()
                if let count = lensCounts[lens] {
                    Text("\(count)").font(Typography.caption()).foregroundStyle(c.textSecondary)
                }
            }
            .padding(Space.sm)
            .background(isActive ? c.bgSelected : Color.clear)
            .overlay(alignment: .leading) {
                if isActive {
                    Rectangle().fill(c.accentDefault).frame(width: 2)
                }
            }
            .clipShape(RoundedRectangle(cornerRadius: Radius.md))
        }
        .buttonStyle(.plain)
    }

    private var logo: some View {
        Circle().strokeBorder(c.accentDefault, lineWidth: 1.4).frame(width: 20, height: 20)
            .overlay(Circle().strokeBorder(c.accentHover, lineWidth: 1.4).frame(width: 11, height: 11))
    }

    private var searchBar: some View {
        HStack(spacing: Space.sm) {
            Image(systemName: "magnifyingglass").foregroundStyle(c.textSecondary).font(.system(size: 11))
            Text("Search vault…").font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
            Spacer()
            Text("⌘K").font(Typography.caption()).foregroundStyle(c.textDisabled)
        }
        .padding(Space.sm)
        .background(c.bgHover)
        .clipShape(RoundedRectangle(cornerRadius: Radius.md))
        .overlay(RoundedRectangle(cornerRadius: Radius.md).stroke(c.borderDefault, lineWidth: 1))
    }

    private func section(title: String, dotColor: Color, nodes: [CachedNode]) -> some View {
        VStack(alignment: .leading, spacing: 1) {
            HStack(spacing: Space.xs) {
                Circle().fill(dotColor).frame(width: 6, height: 6)
                Text(title.uppercased()).font(Typography.caption()).foregroundStyle(c.textSecondary)
            }
            .padding(.bottom, Space.xs)

            if nodes.isEmpty {
                Text("None yet").font(Typography.bodySmall()).foregroundStyle(c.textDisabled).padding(Space.sm)
            }
            ForEach(nodes, id: \.id) { node in
                sidebarRow(node)
            }
        }
    }

    private func sidebarRow(_ node: CachedNode) -> some View {
        let isActive = node.path == openNode?.path
        return Button {
            selectedLens = nil
            openNode = node
        } label: {
            Text(titleFor(node))
                .font(Typography.bodySans())
                .foregroundStyle(c.textPrimary)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(Space.sm)
                .background(isActive ? c.bgSelected : Color.clear)
                .overlay(alignment: .leading) {
                    if isActive {
                        Rectangle().fill(c.accentDefault).frame(width: 2)
                    }
                }
                .clipShape(RoundedRectangle(cornerRadius: Radius.md))
        }
        .buttonStyle(.plain)
    }

    /// No dedicated `title` field exists in the schema yet (`search.rs`'s own
    /// documented simplification) — the file's basename stands in, same
    /// stand-in `search.rs` already uses.
    private func titleFor(_ node: CachedNode) -> String {
        (node.path as NSString).lastPathComponent.replacingOccurrences(of: ".md", with: "")
    }

    private var graphButton: some View {
        HStack(spacing: Space.sm) {
            Image(systemName: "circle.grid.cross").font(.system(size: 12)).foregroundStyle(c.textSecondary)
            Text("Open graph").font(Typography.bodySmall()).foregroundStyle(c.textPrimary)
            Spacer()
            Text("⌘G").font(Typography.caption()).foregroundStyle(c.textDisabled)
        }
        .padding(Space.sm)
        .overlay(RoundedRectangle(cornerRadius: Radius.md).stroke(c.borderDefault, lineWidth: 1))
    }

    private func reload() {
        projects = (try? engine.search(query: "", nodeType: "project", domain: nil, tag: nil)) ?? []
        areas = (try? engine.search(query: "", nodeType: "area", domain: nil, tag: nil)) ?? []
        resources = (try? engine.search(query: "", nodeType: "resource", domain: nil, tag: nil)) ?? []

        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd"
        let today = formatter.string(from: Date())
        lensCounts = [
            .inbox: (try? engine.inbox())?.count ?? 0,
            .today: (try? engine.today(today: today))?.count ?? 0,
            .upcoming: (try? engine.upcoming(from: today, days: 7))?.count ?? 0,
            .somedayMaybe: (try? engine.somedayMaybe())?.count ?? 0,
            .logbook: (try? engine.logbook())?.count ?? 0,
        ]
    }
}
