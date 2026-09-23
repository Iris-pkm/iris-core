import SwiftUI
import IrisCore

/// The seven "standing" sidebar destinations (Trash/History/Spaces/Today/
/// Reading List/Music Ideas/Calendar) as one selection, not seven
/// independent booleans. They used to be separate `@State` bools on
/// `AppShell`, each row clearing only *some* of its siblings by hand —
/// `showCalendar` in particular got missed by nearly every other row's
/// handler, so several sidebar rows could show as "active" simultaneously
/// even though the content pane (an if/else chain) only ever rendered one.
/// One enum, shared by `AppShell` and `Sidebar`, makes that class of bug
/// structurally impossible, matching how `selectedLens`/`workbenchCategory`
/// already work.
fileprivate enum AuxScreen {
    case trash, history, spaces, dailyNote, readingList, musicIdeas, calendar, plugins
}

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
    /// The seven "standing" sidebar destinations (Trash/History/Spaces/
    /// Today/Reading List/Music Ideas/Calendar) as one selection, not seven
    /// independent booleans. They used to be separate `@State` bools, each
    /// row clearing only *some* of its siblings by hand — `showCalendar`
    /// in particular got missed by nearly every other row's handler, so
    /// several sidebar rows could show as "active" simultaneously even
    /// though the content pane (an if/else chain) only ever rendered one.
    /// One enum makes that class of bug structurally impossible, matching
    /// how `selectedLens`/`workbenchCategory` already work.
    @State private var auxScreen: AuxScreen?
    @State private var activeSpaceID: String?
    @State private var recentCaptures: [CaptureItem] = []
    /// Graph (Phase 6, pulled forward at the user's explicit request — see
    /// `GraphView`'s own doc comment) is a full replacement, not a sibling
    /// selection or an overlay — `screen-flow.md`'s own note: "treat as
    /// full replace regardless." Centered on whatever node is currently
    /// open; the sidebar's "Open graph" button and ⌘G are both disabled
    /// (shown, not hidden — same honest-disabled pattern as Onboarding's
    /// unbuilt import sources) when nothing is open, rather than silently
    /// doing nothing on click.
    @State private var showGraph = false
    /// Timeline (Phase 3, pulled forward alongside Graph — see
    /// `TimelineView`'s own doc comment) is the same full-replacement
    /// treatment as Graph, for the same reason: its own mockup has no
    /// PARA/nav sidebar, just a bespoke critical-path toggle and legend.
    /// Unlike Graph it needs no center node, so it's reachable
    /// unconditionally from a standing sidebar row — this also resolves
    /// `screen-flow.md`'s flagged "entry trigger TBD" for this screen.
    @State private var showTimeline = false

    init(engine: FfiEngine, initialLens: TaskLens? = nil) {
        self.engine = engine
        _selectedLens = State(initialValue: initialLens)
    }

    var body: some View {
        if showGraph, let openNode {
            GraphView(
                engine: engine, centerNode: openNode,
                onOpenNode: { node in
                    showGraph = false
                    self.openNode = node
                },
                onBack: { showGraph = false }
            )
            .frame(minWidth: 900, idealWidth: 1280, minHeight: 640, idealHeight: 800)
        } else if showTimeline {
            TimelineView(
                engine: engine,
                onOpenNode: { node in
                    showTimeline = false
                    openNode = node
                },
                onBack: { showTimeline = false }
            )
            .frame(minWidth: 900, idealWidth: 1280, minHeight: 640, idealHeight: 800)
        } else {
        ZStack {
            HStack(spacing: 0) {
                Sidebar(
                    engine: engine,
                    openNode: $openNode,
                    selectedLens: $selectedLens,
                    workbenchCategory: $workbenchCategory,
                    auxScreen: $auxScreen,
                    showSearch: { showSearch = true },
                    showGraph: { showGraph = true },
                    showTimeline: { showTimeline = true }
                )
                Divider().background(c.borderDefault)

                if let auxScreen {
                    auxScreenView(for: auxScreen)
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

            Button("", action: { if openNode != nil { showGraph = true } })
                .keyboardShortcut("g", modifiers: .command)
                .opacity(0)
                .frame(width: 0, height: 0)
                .accessibilityHidden(true)
        }
        .background(c.bgCanvas)
        .frame(minWidth: 900, idealWidth: 1280, minHeight: 640, idealHeight: 800)
        }
    }

    @ViewBuilder
    private func auxScreenView(for screen: AuxScreen) -> some View {
        switch screen {
        case .musicIdeas: MusicIdeasView(engine: engine)
        case .readingList: ReadingListView(engine: engine)
        case .dailyNote: DailyNoteView(engine: engine)
        case .spaces: SpacesView(engine: engine, activeSpaceID: $activeSpaceID)
        case .history: HistoryView(engine: engine)
        case .trash: TrashView(engine: engine)
        case .calendar: CalendarView(engine: engine)
        case .plugins: PluginsView(engine: engine)
        }
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
    @Binding var auxScreen: AuxScreen?
    let showSearch: () -> Void
    let showGraph: () -> Void
    let showTimeline: () -> Void
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
            calendarRow
            timelineRow

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
            pluginsRow

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
            auxScreen = nil
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
            .contentShape(Rectangle())
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
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .keyboardShortcut("k", modifiers: .command)
    }

    /// Shared row shape for the seven `AuxScreen` sidebar destinations —
    /// one implementation instead of seven near-identical `Button`s that
    /// each drifted slightly (this is exactly how `showCalendar` got left
    /// out of several of the old handlers).
    private func auxScreenRow(_ screen: AuxScreen, icon: String, title: String, trailingCount: Int? = nil) -> some View {
        let isActive = auxScreen == screen
        return Button {
            selectedLens = nil
            workbenchCategory = nil
            openNode = nil
            auxScreen = screen
        } label: {
            HStack(spacing: Space.sm) {
                Image(systemName: icon).font(.system(size: 12))
                Text(title).font(Typography.bodySans())
                Spacer()
                if let trailingCount {
                    Text("\(trailingCount)").font(Typography.caption())
                }
            }
            .foregroundStyle(isActive ? c.accentDefault : c.textPrimary)
            .padding(Space.sm)
            .background(isActive ? c.bgSelected : Color.clear)
            .overlay(alignment: .leading) {
                if isActive { Rectangle().fill(c.accentDefault).frame(width: 2) }
            }
            .clipShape(RoundedRectangle(cornerRadius: Radius.md))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    private var dailyNoteRow: some View { auxScreenRow(.dailyNote, icon: "calendar", title: "Today") }
    private var readingListRow: some View { auxScreenRow(.readingList, icon: "books.vertical", title: "Reading List") }
    private var musicIdeasRow: some View { auxScreenRow(.musicIdeas, icon: "music.note", title: "Music Ideas") }
    private var calendarRow: some View { auxScreenRow(.calendar, icon: "calendar.badge.clock", title: "Calendar") }
    private var trashRow: some View { auxScreenRow(.trash, icon: "trash", title: "Trash", trailingCount: trashCount) }
    private var historyRow: some View { auxScreenRow(.history, icon: "clock", title: "History") }
    private var spacesRow: some View { auxScreenRow(.spaces, icon: "square.3.layers.3d", title: "Spaces") }
    private var pluginsRow: some View { auxScreenRow(.plugins, icon: "puzzlepiece.extension", title: "Plugins") }

    private func section(title: String, category: PARAWorkbenchView.Category, dotColor: Color, nodes: [CachedNode]) -> some View {
        VStack(alignment: .leading, spacing: 1) {
            Button {
                selectedLens = nil
                openNode = nil
                auxScreen = nil
                workbenchCategory = category
            } label: {
                HStack(spacing: Space.xs) {
                    Circle().fill(dotColor).frame(width: 6, height: 6)
                    Text(title.uppercased()).font(Typography.caption()).foregroundStyle(c.textSecondary)
                }
                .padding(.bottom, Space.xs)
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
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
            auxScreen = nil
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
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    /// No dedicated `title` field exists in the schema yet (`search.rs`'s own
    /// documented simplification) — the file's basename stands in, same
    /// stand-in `search.rs` already uses.
    private func titleFor(_ node: CachedNode) -> String {
        (node.path as NSString).lastPathComponent.replacingOccurrences(of: ".md", with: "")
    }

    /// Timeline was pulled forward from Phase 3 alongside Graph — resolves
    /// `screen-flow.md`'s flagged "entry trigger TBD" for this screen.
    /// Unconditionally enabled (unlike Graph, a project timeline needs no
    /// single center node).
    private var timelineRow: some View {
        Button(action: showTimeline) {
            HStack(spacing: Space.sm) {
                Image(systemName: "chart.bar.xaxis").font(.system(size: 12))
                Text("Timeline").font(Typography.bodySans())
                Spacer()
            }
            .foregroundStyle(c.textPrimary)
            .padding(Space.sm)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    /// Real now (Graph was pulled forward from Phase 6) — but only when a
    /// node is open, since the graph needs a center. Shown disabled rather
    /// than hidden when nothing's open, same honest-disabled pattern as
    /// Onboarding's unbuilt import sources, rather than a click that
    /// silently does nothing.
    private var graphButton: some View {
        Button(action: showGraph) {
            HStack(spacing: Space.sm) {
                Image(systemName: "circle.grid.cross").font(.system(size: 12))
                Text("Open graph").font(Typography.bodySmall())
                Spacer()
                Text("⌘G").font(Typography.caption()).foregroundStyle(c.textDisabled)
            }
            .foregroundStyle(openNode == nil ? c.textDisabled : c.textPrimary)
            .padding(Space.sm)
            .contentShape(Rectangle())
            .overlay(RoundedRectangle(cornerRadius: Radius.md).stroke(c.borderDefault, lineWidth: 1))
        }
        .buttonStyle(.plain)
        .disabled(openNode == nil)
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
