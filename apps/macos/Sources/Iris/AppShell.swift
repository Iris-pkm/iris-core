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
    /// The PARA workbench (`design/canvas/PARAWorkbench.dc.html`) is a
    /// third sibling selection, entered by clicking a sidebar section
    /// *header* ("Projects"/"Areas"/"Resources") — distinct from clicking
    /// an individual item underneath it, which still opens that node
    /// directly via `openNode`. All three selections are mutually exclusive.
    @State private var workbenchCategory: PARAWorkbenchView.Category?
    @State private var showSearch = false
    @State private var showQuickCapture = false
    @State private var showTrash = false
    @State private var showHistory = false
    @State private var showSpaces = false
    @State private var activeSpaceID: String?
    @State private var showDailyNote = false
    @State private var showReadingList = false
    @State private var showMusicIdeas = false
    @State private var recentCaptures: [CaptureItem] = []

    var body: some View {
        ZStack {
            HStack(spacing: 0) {
                Sidebar(
                    engine: engine,
                    openNode: $openNode,
                    selectedLens: $selectedLens,
                    workbenchCategory: $workbenchCategory,
                    showTrash: $showTrash,
                    showHistory: $showHistory,
                    showSpaces: $showSpaces,
                    showDailyNote: $showDailyNote,
                    showReadingList: $showReadingList,
                    showMusicIdeas: $showMusicIdeas,
                    showSearch: { showSearch = true }
                )
                Divider().background(c.borderDefault)

                if showMusicIdeas {
                    MusicIdeasView(engine: engine)
                } else if showReadingList {
                    ReadingListView(engine: engine)
                } else if showDailyNote {
                    DailyNoteView(engine: engine)
                } else if showSpaces {
                    SpacesView(engine: engine, activeSpaceID: $activeSpaceID)
                } else if showHistory {
                    HistoryView(engine: engine)
                } else if showTrash {
                    TrashView(engine: engine)
                } else if let selectedLens {
                    lensView(for: selectedLens)
                } else if let workbenchCategory {
                    PARAWorkbenchView(engine: engine, initialCategory: workbenchCategory, onOpenNode: { node in
                        self.workbenchCategory = nil
                        openNode = node
                    })
                } else if let openNode {
                    detail(for: openNode)
                } else {
                    emptyState
                }
            }

            if showSearch {
                SearchPaletteView(engine: engine, dismiss: { showSearch = false }) { node in
                    openNode = node
                    showSearch = false
                }
            }

            if showQuickCapture {
                QuickCaptureView(
                    engine: engine,
                    recentCaptures: recentCaptures,
                    dismiss: { showQuickCapture = false }
                ) { item in
                    recentCaptures.insert(item, at: 0)
                    showQuickCapture = false
                }
            }

            // A mounted command is the small SwiftUI-native bridge from the
            // documented shortcut to this global overlay. A true outside-the-
            // app hotkey needs the later NSPanel/event-monitoring integration.
            Button("", action: { showQuickCapture = true })
                .keyboardShortcut("c", modifiers: [.command, .shift])
                .opacity(0)
                .frame(width: 0, height: 0)
                .accessibilityHidden(true)
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
        case .reminders: RemindersView(engine: engine)
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
            ProjectView(engine: engine, relPath: node.path, onOpenNode: { openNode = $0 })
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if node.nodeType == "area" && nodeTitle(node).caseInsensitiveCompare("Trading") == .orderedSame {
            TradingJournalView(engine: engine)
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

    private func nodeTitle(_ node: CachedNode) -> String {
        (node.path as NSString).lastPathComponent.replacingOccurrences(of: ".md", with: "")
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
    @Binding var workbenchCategory: PARAWorkbenchView.Category?
    @Binding var showTrash: Bool
    @Binding var showHistory: Bool
    @Binding var showSpaces: Bool
    @Binding var showDailyNote: Bool
    @Binding var showReadingList: Bool
    @Binding var showMusicIdeas: Bool
    let showSearch: () -> Void
    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    @State private var projects: [CachedNode] = []
    @State private var areas: [CachedNode] = []
    @State private var resources: [CachedNode] = []
    @State private var lensCounts: [TaskLens: Int] = [:]
    @State private var trashCount = 0

    var body: some View {
        VStack(alignment: .leading, spacing: Space.lg) {
            HStack(spacing: Space.sm) {
                logo
                Text("Iris").font(Typography.serif(15, weight: .semibold)).foregroundStyle(c.textPrimary)
            }

            searchBar
            dailyNoteRow
            readingListRow
            musicIdeasRow

            VStack(alignment: .leading, spacing: 1) {
                ForEach(TaskLens.allCases) { lens in
                    lensRow(lens)
                }
            }

            ScrollView {
                VStack(alignment: .leading, spacing: Space.lg) {
                    section(title: "Projects", category: .projects, dotColor: c.warning, nodes: projects)
                    section(title: "Areas", category: .areas, dotColor: c.success, nodes: areas)
                    section(title: "Resources", category: .resources, dotColor: c.accentDefault, nodes: resources)
                }
            }

            trashRow
            historyRow
            spacesRow

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
            workbenchCategory = nil
            showTrash = false
            showHistory = false
            showSpaces = false
            showDailyNote = false
            showReadingList = false
            showMusicIdeas = false
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
        Button(action: showSearch) {
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
        .buttonStyle(.plain)
        .keyboardShortcut("k", modifiers: .command)
    }

    private var dailyNoteRow: some View {
        Button {
            selectedLens = nil; workbenchCategory = nil; openNode = nil
            showTrash = false; showHistory = false; showSpaces = false; showDailyNote = true
        } label: {
            HStack(spacing: Space.sm) {
                Image(systemName: "calendar").font(.system(size: 12))
                Text("Today").font(Typography.bodySans())
                Spacer()
            }
            .foregroundStyle(showDailyNote ? c.accentDefault : c.textPrimary)
            .padding(Space.sm).background(showDailyNote ? c.bgSelected : Color.clear)
            .overlay(alignment: .leading) { if showDailyNote { Rectangle().fill(c.accentDefault).frame(width: 2) } }
            .clipShape(RoundedRectangle(cornerRadius: Radius.md))
        }.buttonStyle(.plain)
    }

    private var readingListRow: some View {
        Button {
            selectedLens = nil; workbenchCategory = nil; openNode = nil
            showTrash = false; showHistory = false; showSpaces = false; showDailyNote = false; showReadingList = true
        } label: {
            HStack(spacing: Space.sm) {
                Image(systemName: "books.vertical").font(.system(size: 12))
                Text("Reading List").font(Typography.bodySans())
                Spacer()
            }.foregroundStyle(showReadingList ? c.accentDefault : c.textPrimary)
                .padding(Space.sm).background(showReadingList ? c.bgSelected : Color.clear)
                .overlay(alignment: .leading) { if showReadingList { Rectangle().fill(c.accentDefault).frame(width: 2) } }
                .clipShape(RoundedRectangle(cornerRadius: Radius.md))
        }.buttonStyle(.plain)
    }

    private var musicIdeasRow: some View {
        Button {
            selectedLens = nil; workbenchCategory = nil; openNode = nil
            showTrash = false; showHistory = false; showSpaces = false; showDailyNote = false; showReadingList = false; showMusicIdeas = true
        } label: {
            HStack(spacing: Space.sm) { Image(systemName: "music.note").font(.system(size: 12)); Text("Music Ideas").font(Typography.bodySans()); Spacer() }
                .foregroundStyle(showMusicIdeas ? c.accentDefault : c.textPrimary).padding(Space.sm).background(showMusicIdeas ? c.bgSelected : Color.clear)
                .overlay(alignment: .leading) { if showMusicIdeas { Rectangle().fill(c.accentDefault).frame(width: 2) } }.clipShape(RoundedRectangle(cornerRadius: Radius.md))
        }.buttonStyle(.plain)
    }

    private func section(title: String, category: PARAWorkbenchView.Category, dotColor: Color, nodes: [CachedNode]) -> some View {
        VStack(alignment: .leading, spacing: 1) {
            Button {
                selectedLens = nil
                openNode = nil
                showTrash = false
                showHistory = false
                showSpaces = false
                workbenchCategory = category
            } label: {
                HStack(spacing: Space.xs) {
                    Circle().fill(dotColor).frame(width: 6, height: 6)
                    Text(title.uppercased()).font(Typography.caption()).foregroundStyle(c.textSecondary)
                }
                .padding(.bottom, Space.xs)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .buttonStyle(.plain)

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
            workbenchCategory = nil
            showTrash = false
            showHistory = false
            showSpaces = false
            showDailyNote = false
            showReadingList = false
            showMusicIdeas = false
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

    private var trashRow: some View {
        Button {
            selectedLens = nil
            workbenchCategory = nil
            openNode = nil
            showTrash = true
            showHistory = false
            showSpaces = false
            showSpaces = false
        } label: {
            HStack(spacing: Space.sm) {
                Image(systemName: "trash").font(.system(size: 12))
                Text("Trash").font(Typography.bodySans())
                Spacer()
                Text("\(trashCount)").font(Typography.caption())
            }
            .foregroundStyle(showTrash ? c.accentDefault : c.textPrimary)
            .padding(Space.sm)
            .background(showTrash ? c.bgSelected : Color.clear)
            .overlay(alignment: .leading) {
                if showTrash { Rectangle().fill(c.accentDefault).frame(width: 2) }
            }
            .clipShape(RoundedRectangle(cornerRadius: Radius.md))
        }
        .buttonStyle(.plain)
    }

    private var historyRow: some View {
        Button {
            selectedLens = nil
            workbenchCategory = nil
            openNode = nil
            showTrash = false
            showHistory = true
            showSpaces = false
        } label: {
            HStack(spacing: Space.sm) {
                Image(systemName: "clock").font(.system(size: 12))
                Text("History").font(Typography.bodySans())
                Spacer()
            }
            .foregroundStyle(showHistory ? c.accentDefault : c.textPrimary)
            .padding(Space.sm)
            .background(showHistory ? c.bgSelected : Color.clear)
            .overlay(alignment: .leading) {
                if showHistory { Rectangle().fill(c.accentDefault).frame(width: 2) }
            }
            .clipShape(RoundedRectangle(cornerRadius: Radius.md))
        }
        .buttonStyle(.plain)
    }

    private var spacesRow: some View {
        Button {
            selectedLens = nil; workbenchCategory = nil; openNode = nil
            showTrash = false; showHistory = false; showSpaces = true
        } label: {
            HStack(spacing: Space.sm) {
                Image(systemName: "square.3.layers.3d").font(.system(size: 12))
                Text("Spaces").font(Typography.bodySans())
                Spacer()
            }
            .foregroundStyle(showSpaces ? c.accentDefault : c.textPrimary)
            .padding(Space.sm).background(showSpaces ? c.bgSelected : Color.clear)
            .overlay(alignment: .leading) { if showSpaces { Rectangle().fill(c.accentDefault).frame(width: 2) } }
            .clipShape(RoundedRectangle(cornerRadius: Radius.md))
        }
        .buttonStyle(.plain)
    }

    private func reload() {
        projects = (try? engine.search(query: "", nodeType: "project", domain: nil, tag: nil)) ?? []
        areas = (try? engine.search(query: "", nodeType: "area", domain: nil, tag: nil)) ?? []
        resources = (try? engine.search(query: "", nodeType: "resource", domain: nil, tag: nil)) ?? []
        trashCount = (try? engine.trash())?.count ?? 0

        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd"
        let today = formatter.string(from: Date())
        lensCounts = [
            .inbox: (try? engine.inbox())?.count ?? 0,
            .today: (try? engine.today(today: today))?.count ?? 0,
            .upcoming: (try? engine.upcoming(from: today, days: 7))?.count ?? 0,
            .somedayMaybe: (try? engine.somedayMaybe())?.count ?? 0,
            .logbook: (try? engine.logbook())?.count ?? 0,
            .reminders: (try? engine.search(query: "", nodeType: "reminder", domain: nil, tag: nil))?.count ?? 0,
        ]
    }
}
