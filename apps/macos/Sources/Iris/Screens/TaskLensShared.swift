import SwiftUI
import IrisCore

/// The five task-view lenses (`design/canvas/Inbox.dc.html` /
/// `Today.dc.html` / `Upcoming.dc.html` / `SomedayMaybe.dc.html` /
/// `Logbook.dc.html`) are filtered views over the same task nodes
/// (ARCHITECTURE.md §12) — this file holds what all five share: the
/// sidebar's routing enum, the row layout, and a project-name lookup
/// (`CachedNode.parentProject` is only an id; every lens's right rail
/// needs the human name for its "By project" breakdown).
enum TaskLens: String, CaseIterable, Identifiable {
    case inbox, today, upcoming, somedayMaybe, logbook, reminders
    var id: String { rawValue }

    var title: String {
        switch self {
        case .inbox: return "Inbox"
        case .today: return "Today"
        case .upcoming: return "Upcoming"
        case .somedayMaybe: return "Someday/Maybe"
        case .logbook: return "Logbook"
        case .reminders: return "Reminders"
        }
    }
}

func taskTitleFor(_ node: CachedNode) -> String {
    (node.path as NSString).lastPathComponent.replacingOccurrences(of: ".md", with: "")
}

/// `parent_project` is an id on `CachedNode` — resolving it to a display
/// name means fetching every project once and mapping id -> title, the
/// same basename stand-in every other screen uses.
func projectTitleMap(engine: FfiEngine) -> [String: String] {
    let projects = (try? engine.search(query: "", nodeType: "project", domain: nil, tag: nil)) ?? []
    var map: [String: String] = [:]
    for project in projects {
        map[project.id] = taskTitleFor(project)
    }
    return map
}

func projectLabel(_ node: CachedNode, titles: [String: String]) -> String {
    guard let projectId = node.parentProject, let title = titles[projectId] else { return "No project" }
    return title
}

func priorityLabel(_ priority: String?) -> String? {
    switch priority {
    case "urgent": return "Urgent"
    case "high": return "High"
    case "normal": return "Normal"
    case "low": return "Low"
    default: return nil
    }
}

/// File a task to a project (Inbox's "File…") — adds a `parent_project`
/// relation via the existing `updateNode`. No dedicated FFI method exists
/// for this (or needs to: it's a one-relation mutation on an already-loaded
/// node, exactly what `updateNode` is for).
func fileToProject(engine: FfiEngine, relPath: String, projectId: String) {
    guard let parsed = try? engine.readNode(relPath: relPath) else { return }
    var node = parsed.node
    node.relations.append(Relation(relType: "parent_project", target: projectId))
    _ = try? engine.updateNode(relPath: relPath, node: node)
}

/// Someday/Maybe's "Schedule…" — sets `scheduled_date` via `updateNode`.
func scheduleTask(engine: FfiEngine, relPath: String, date: Date) {
    guard let parsed = try? engine.readNode(relPath: relPath) else { return }
    var node = parsed.node
    let formatter = DateFormatter()
    formatter.dateFormat = "yyyy-MM-dd"
    node.scheduledDate = formatter.string(from: date)
    _ = try? engine.updateNode(relPath: relPath, node: node)
}

/// Today/Logbook's checkbox: marking done goes through the real
/// `completeTask` (handles recurrence); reopening (Logbook's checkmark,
/// Today unchecking a scheduled item) has no dedicated "reopen" method —
/// there's nothing recurrence-specific to reverse, it's just clearing
/// `status` back to unset via `updateNode`, so no new backend surface was
/// added for it.
func toggleTaskDone(engine: FfiEngine, relPath: String, currentlyDone: Bool) {
    if currentlyDone {
        guard let parsed = try? engine.readNode(relPath: relPath) else { return }
        var node = parsed.node
        node.status = nil
        _ = try? engine.updateNode(relPath: relPath, node: node)
    } else {
        _ = try? engine.completeTask(relPath: relPath)
    }
}

/// Shared right-rail chrome (240px, `bgSurface`, 16px padding, sections
/// separated by a 1px divider) — every lens's rail is this shape with
/// different content inside.
struct TaskLensRail<Content: View>: View {
    let content: Content
    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    init(@ViewBuilder content: () -> Content) { self.content = content() }

    var body: some View {
        VStack(alignment: .leading, spacing: Space.lg) {
            content
            Spacer()
        }
        .padding(Space.lg)
        .frame(width: 240)
        .frame(maxHeight: .infinity)
        .background(c.bgSurface)
    }
}

struct TaskLensRailSection<Content: View>: View {
    let title: String
    let content: Content
    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    init(_ title: String, @ViewBuilder content: () -> Content) {
        self.title = title
        self.content = content()
    }

    var body: some View {
        VStack(alignment: .leading, spacing: Space.sm) {
            Text(title.uppercased()).font(Typography.caption()).foregroundStyle(c.textSecondary)
            content
        }
    }
}

struct TaskLensRailDivider: View {
    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }
    var body: some View { Rectangle().fill(c.borderDefault).frame(height: 1) }
}

struct TaskLensCountRow: View {
    let label: String
    let count: Int
    var dotColor: Color?
    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    var body: some View {
        HStack {
            HStack(spacing: Space.xs) {
                if let dotColor {
                    Circle().fill(dotColor).frame(width: 7, height: 7)
                }
                Text(label).font(Typography.bodySmall()).foregroundStyle(c.textPrimary)
            }
            Spacer()
            Text("\(count)").font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
        }
    }
}

/// The row shape every lens shares: an optional checkbox (interactive on
/// Today/Logbook, decorative elsewhere), title + meta line, an optional
/// priority pill, and an optional trailing action ("File…"/"Schedule…").
struct TaskLensRow<Trailing: View>: View {
    let title: String
    let meta: String
    var titleStruckThrough: Bool = false
    var priority: String? = nil
    var trailingTag: String? = nil
    var checkbox: Checkbox? = nil
    var onOpen: (() -> Void)? = nil
    /// Arbitrary trailing content — a plain "File…"/"Schedule…" `Button`,
    /// or (Inbox/Someday-Maybe) a real `Menu`/`DatePicker` control, since a
    /// plain click-action closure can't itself show a picker.
    @ViewBuilder var trailing: () -> Trailing

    struct Checkbox {
        let checked: Bool
        let toggle: () -> Void
    }

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    var body: some View {
        HStack(spacing: Space.md) {
            if let checkbox {
                Button(action: checkbox.toggle) {
                    ZStack {
                        RoundedRectangle(cornerRadius: 4)
                            .strokeBorder(checkbox.checked ? c.success : c.borderStrong, lineWidth: 1.5)
                            .background(RoundedRectangle(cornerRadius: 4).fill(checkbox.checked ? c.success : Color.clear))
                            .frame(width: 16, height: 16)
                        if checkbox.checked {
                            Image(systemName: "checkmark").font(.system(size: 9, weight: .bold)).foregroundStyle(c.bgCanvas)
                        }
                    }
                }
                .buttonStyle(.plain)
            }

            Button {
                onOpen?()
            } label: {
                VStack(alignment: .leading, spacing: 2) {
                    Text(title)
                        .font(Typography.sans(14, weight: .medium))
                        .foregroundStyle(titleStruckThrough ? c.textSecondary : c.textPrimary)
                        .strikethrough(titleStruckThrough)
                    Text(meta).font(Typography.caption()).foregroundStyle(c.textSecondary)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .buttonStyle(.plain)
            .disabled(onOpen == nil)

            if let priority, let label = priorityLabel(priority) {
                Text(label)
                    .font(.system(size: 11))
                    .padding(.horizontal, Space.sm)
                    .padding(.vertical, 2)
                    .background(priorityColor(priority).opacity(0.15))
                    .foregroundStyle(priorityColor(priority))
                    .clipShape(Capsule())
            }

            if let trailingTag {
                Text(trailingTag)
                    .font(.system(size: 11))
                    .padding(.horizontal, Space.sm)
                    .padding(.vertical, 2)
                    .background(c.accentTint)
                    .foregroundStyle(c.accentDefault)
                    .clipShape(Capsule())
            }

            trailing()
        }
        .padding(.vertical, Space.md)
        .overlay(alignment: .bottom) { Rectangle().fill(c.borderDefault).frame(height: 1) }
    }

    private func priorityColor(_ p: String) -> Color {
        switch p {
        case "urgent": return c.danger
        case "high": return c.warning
        case "low": return c.textDisabled
        default: return c.textSecondary
        }
    }
}

extension TaskLensRow where Trailing == EmptyView {
    init(
        title: String, meta: String, titleStruckThrough: Bool = false,
        priority: String? = nil, trailingTag: String? = nil,
        checkbox: Checkbox? = nil, onOpen: (() -> Void)? = nil
    ) {
        self.init(
            title: title, meta: meta, titleStruckThrough: titleStruckThrough,
            priority: priority, trailingTag: trailingTag, checkbox: checkbox, onOpen: onOpen,
            trailing: { EmptyView() }
        )
    }
}

/// Shared "N items" header + description line every lens opens with.
struct TaskLensHeader: View {
    let title: String
    let trailing: String
    let description: String
    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    var body: some View {
        VStack(alignment: .leading, spacing: Space.xs) {
            HStack(alignment: .lastTextBaseline) {
                Text(title).font(Typography.h1()).foregroundStyle(c.textPrimary)
                Spacer()
                Text(trailing).font(Typography.caption()).foregroundStyle(c.textSecondary)
            }
            Text(description).font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
        }
    }
}

struct TaskLensEmptyState: View {
    let title: String
    let message: String
    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    var body: some View {
        VStack(spacing: Space.xs) {
            Text(title).font(Typography.serif(17, weight: .semibold)).foregroundStyle(c.textPrimary)
            Text(message)
                .font(Typography.bodySmall())
                .foregroundStyle(c.textSecondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 320)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, Space.xxxl)
    }
}
