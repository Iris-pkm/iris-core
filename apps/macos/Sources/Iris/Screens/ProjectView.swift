import SwiftUI
import IrisCore

/// A Project's own detail route (`design/canvas/GuidedActivation.dc.html`,
/// `design/navigation.md`'s "Project detail / Guided Activation" row,
/// ADR-023) — not the generic Node Editor: replaces the *entire* content
/// pane (no Connections/Dev Mode rail, confirmed by the mockup only ever
/// showing sidebar + one content column here), with a status control that
/// swaps the whole body between the normal project view and the Guided
/// Activation dashboard while `project_status == active`.
struct ProjectView: View {
    let engine: FfiEngine
    let relPath: String
    let onOpenNode: (CachedNode) -> Void

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }

    @State private var parsed: FfiParsedNode?
    @State private var environment: ActivationEnvironment?
    @State private var blockerNames: [String: String] = [:]
    /// "View all" on the Distillation queue card pushes here — a local nav
    /// state rather than routing through `AppShell`, since this is purely
    /// a drill-down within the Project's own route (mockup: breadcrumb
    /// reads "<Project> / Distillation Queue", same window, no shell change).
    @State private var showDistillationQueue = false
    /// The header's "Tasks" link pushes here — same local-nav pattern
    /// (`design/canvas/Tasks.dc.html`'s breadcrumb: "<Project> / Tasks").
    @State private var showTasks = false

    var body: some View {
        Group {
            if showDistillationQueue, let parsed {
                DistillationQueueView(
                    engine: engine,
                    projectId: parsed.node.id,
                    projectTitle: titleFor(relPath: relPath),
                    onBack: { showDistillationQueue = false }
                )
            } else if showTasks, let parsed {
                ProjectTasksView(
                    engine: engine,
                    projectId: parsed.node.id,
                    projectTitle: titleFor(relPath: relPath),
                    onBack: { showTasks = false },
                    onOpenTask: onOpenNode
                )
            } else {
                ScrollView {
                    VStack(alignment: .leading, spacing: 0) {
                        if let parsed {
                            header(for: parsed.node)

                            if parsed.node.projectStatus == .active {
                                activationBanner
                                activationGrid
                                recentlyAdded
                            } else {
                                normalLayout(parsed)
                            }
                        }
                    }
                }
                .background(c.bgCanvas)
            }
        }
        .id(relPath)
        .task(id: relPath) { load() }
    }

    // MARK: - Header (shared by both layouts)

    private func header(for node: FfiNode) -> some View {
        VStack(alignment: .leading, spacing: Space.md) {
            HStack(alignment: .lastTextBaseline, spacing: Space.md) {
                Text(titleFor(node)).font(Typography.h1()).foregroundStyle(c.textPrimary)
                Text("PROJECT")
                    .font(Typography.caption())
                    .padding(.horizontal, Space.sm)
                    .padding(.vertical, Space.xxs)
                    .background(c.bgHover)
                    .clipShape(Capsule())
                    .foregroundStyle(c.textSecondary)
                Spacer()
                Button {
                    showTasks = true
                } label: {
                    Text("Tasks \u{2192}").font(Typography.bodySmall()).foregroundStyle(c.accentDefault)
                }
                .buttonStyle(.plain)
            }

            HStack(spacing: Space.md) {
                Text("STATUS").font(Typography.caption()).foregroundStyle(c.textSecondary)
                statusSegmented(current: node.projectStatus)
            }
        }
        .padding(EdgeInsets(top: Space.xl, leading: Space.xxl, bottom: Space.lg, trailing: Space.xxl))
        .frame(maxWidth: .infinity, alignment: .leading)
        .overlay(alignment: .bottom) { Rectangle().fill(c.borderDefault).frame(height: 1) }
    }

    /// Only the three statuses the mockup shows as a segmented control.
    /// `someday`/`planned`/`cancelled` projects still open here (their real
    /// status name shown via `normalLayout`'s meta line) but aren't
    /// reachable from this control — `engine.rs`'s own transition rules
    /// don't offer a path back from `cancelled`/`completed` anyway, and
    /// `someday`/`planned` aren't part of the mockup's activation flow.
    private func statusSegmented(current: ProjectStatus?) -> some View {
        HStack(spacing: 2) {
            segment("Active", isActive: current == .active) { setStatus(.active) }
            segment("Paused", isActive: current == .paused) { setStatus(.paused) }
            segment("Completed", isActive: current == .completed) { setStatus(.completed) }
        }
        .padding(2)
        .background(c.bgHover)
        .clipShape(RoundedRectangle(cornerRadius: Radius.md))
    }

    private func segment(_ title: String, isActive: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title)
                .font(Typography.bodySmall())
                .foregroundStyle(isActive ? c.textPrimary : c.textSecondary)
                .padding(.horizontal, Space.md)
                .padding(.vertical, Space.xs)
                .background(isActive ? c.bgSurfaceRaised : Color.clear)
                .clipShape(RoundedRectangle(cornerRadius: Radius.sm))
        }
        .buttonStyle(.plain)
    }

    // MARK: - Guided Activation dashboard

    private var activationBanner: some View {
        Text("Guided Activation — this replaces the normal project view while status is Active (ADR-023). Not a modal, not a separate screen: it's this same route, different content.")
            .font(Typography.bodySmall())
            .foregroundStyle(c.accentDefault)
            .padding(Space.md)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(c.accentTint)
            .clipShape(RoundedRectangle(cornerRadius: Radius.md))
            .padding(EdgeInsets(top: Space.lg, leading: Space.xxl, bottom: 0, trailing: Space.xxl))
    }

    private var activationGrid: some View {
        let columns = [GridItem(.flexible()), GridItem(.flexible()), GridItem(.flexible())]
        return LazyVGrid(columns: columns, spacing: Space.md) {
            if let environment {
                card(
                    "Recommended starting set", count: environment.recommendedStartingSet.count, primary: true,
                    rows: environment.recommendedStartingSet.map { .init(text: titleFor($0), tag: priorityTag($0.priority)) },
                    footer: "Startable now — not done, no unmet depends-on. Ordered by priority."
                )
                Button {
                    showDistillationQueue = true
                } label: {
                    card(
                        "Distillation queue", count: environment.distillationQueue.count,
                        rows: environment.distillationQueue.map { .init(text: titleFor($0)) },
                        footer: "Raw, undistilled notes linked to this project. View all \u{2192}"
                    )
                }
                .buttonStyle(.plain)
                card(
                    "Blocked tasks", count: environment.blockedTasks.count, countColor: c.danger,
                    rows: environment.blockedTasks.map {
                        .init(text: titleFor($0), subtext: blockerNames[$0.id].map { "blocked by: \($0)" })
                    },
                    footer: "An unmet depends-on, either direction."
                )
                card(
                    "Unresolved decisions", count: environment.unresolvedDecisions.count,
                    rows: environment.unresolvedDecisions.map { .init(text: titleFor($0)) },
                    footer: "Open annotation comments — Iris's stand-in for a decision node type."
                )
                card(
                    "Related resources", count: environment.relatedResources.count,
                    rows: environment.relatedResources.map { .init(text: titleFor($0)) }
                )
                card(
                    "Upcoming events", count: environment.upcomingEvents.count,
                    rows: environment.upcomingEvents.map { .init(text: titleFor($0), trailing: shortDate($0.eventStart)) }
                )
            }
        }
        .padding(EdgeInsets(top: Space.md, leading: Space.xxl, bottom: Space.md, trailing: Space.xxl))
    }

    private struct CardRow {
        let text: String
        var tag: (label: String, color: Color)?
        var subtext: String?
        var trailing: String?

        init(text: String, tag: (label: String, color: Color)? = nil, subtext: String? = nil, trailing: String? = nil) {
            self.text = text
            self.tag = tag
            self.subtext = subtext
            self.trailing = trailing
        }
    }

    private func card(
        _ title: String, count: Int, primary: Bool = false, countColor: Color? = nil,
        rows: [CardRow], footer: String? = nil
    ) -> some View {
        VStack(alignment: .leading, spacing: Space.sm) {
            HStack(spacing: Space.sm) {
                Text(title.uppercased()).font(Typography.caption()).foregroundStyle(c.textPrimary)
                Text("\(count)").font(Typography.bodySmall()).foregroundStyle(countColor ?? c.textSecondary)
            }
            VStack(alignment: .leading, spacing: Space.xs) {
                if rows.isEmpty {
                    Text("None").font(Typography.bodySmall()).foregroundStyle(c.textDisabled)
                }
                ForEach(Array(rows.enumerated()), id: \.offset) { _, row in
                    HStack(alignment: .top) {
                        VStack(alignment: .leading, spacing: 2) {
                            HStack(spacing: Space.xs) {
                                if let tag = row.tag {
                                    Text(tag.label)
                                        .font(.system(size: 10))
                                        .padding(.horizontal, Space.xs)
                                        .padding(.vertical, 1)
                                        .background(tag.color.opacity(0.15))
                                        .foregroundStyle(tag.color)
                                        .clipShape(Capsule())
                                }
                                Text(row.text).font(Typography.bodySmall()).foregroundStyle(c.textPrimary)
                            }
                            if let subtext = row.subtext {
                                Text(subtext).font(.system(size: 11)).foregroundStyle(c.danger)
                            }
                        }
                        Spacer()
                        if let trailing = row.trailing {
                            Text(trailing).font(Typography.caption()).foregroundStyle(c.textSecondary)
                        }
                    }
                }
            }
            if let footer {
                Text(footer).font(.system(size: 11)).foregroundStyle(c.textDisabled)
            }
        }
        .padding(Space.md)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(primary ? c.accentTint : c.bgSurface)
        .overlay(RoundedRectangle(cornerRadius: Radius.lg).stroke(primary ? c.accentDefault : c.borderDefault, lineWidth: 1))
        .clipShape(RoundedRectangle(cornerRadius: Radius.lg))
    }

    private var recentlyAdded: some View {
        VStack(alignment: .leading, spacing: Space.sm) {
            HStack(spacing: Space.sm) {
                Text("RECENTLY ADDED").font(Typography.caption()).foregroundStyle(c.textPrimary)
                Text("\(environment?.recentlyAdded.count ?? 0)").font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
            }
            HStack(spacing: Space.sm) {
                ForEach(environment?.recentlyAdded ?? [], id: \.id) { node in
                    Text(titleFor(node))
                        .font(Typography.caption())
                        .foregroundStyle(c.textSecondary)
                        .padding(.horizontal, Space.md)
                        .padding(.vertical, Space.xs)
                        .background(c.bgHover)
                        .clipShape(Capsule())
                }
            }
        }
        .padding(EdgeInsets(top: 0, leading: Space.xxl, bottom: Space.xl, trailing: Space.xxl))
    }

    // MARK: - Normal (non-active) layout

    private func normalLayout(_ parsed: FfiParsedNode) -> some View {
        VStack(alignment: .leading, spacing: Space.md) {
            if !parsed.node.tags.isEmpty {
                HStack(spacing: Space.xs) {
                    ForEach(parsed.node.tags, id: \.self) { tag in
                        Text(tag)
                            .font(Typography.caption())
                            .foregroundStyle(c.textSecondary)
                            .padding(.horizontal, Space.sm)
                            .padding(.vertical, Space.xxs)
                            .background(c.bgHover)
                            .clipShape(Capsule())
                    }
                }
            }
            Text(parsed.body.trimmingCharacters(in: .whitespacesAndNewlines))
                .font(Typography.serif(16))
                .foregroundStyle(c.textPrimary)
                .lineSpacing(6)
                .frame(maxWidth: 640, alignment: .leading)
        }
        .padding(EdgeInsets(top: Space.lg, leading: Space.xxl, bottom: Space.xl, trailing: Space.xxl))
    }

    // MARK: - Helpers

    private func titleFor(_ node: FfiNode) -> String { titleFor(relPath: relPath) }
    private func titleFor(_ node: CachedNode) -> String { titleFor(relPath: node.path) }
    private func titleFor(relPath: String) -> String {
        (relPath as NSString).lastPathComponent.replacingOccurrences(of: ".md", with: "")
    }

    private func priorityTag(_ priority: String?) -> (label: String, color: Color)? {
        switch priority {
        case "urgent": return ("Urgent", c.danger)
        case "high": return ("High", c.warning)
        default: return nil
        }
    }

    private func shortDate(_ rfc3339: String?) -> String? {
        guard let rfc3339, let date = ISO8601DateFormatter().date(from: rfc3339) else { return nil }
        let formatter = DateFormatter()
        formatter.dateFormat = "MMM d"
        return formatter.string(from: date)
    }

    private func setStatus(_ status: ProjectStatus) {
        guard let current = parsed?.node.projectStatus, current != status else { return }
        // Illegal transitions (engine.rs's own rule table) are simply
        // ignored — `load()` re-reads the real, unchanged status either way,
        // so there's nothing for this view to validate ahead of the call.
        _ = try? engine.setProjectStatus(relPath: relPath, status: status)
        load()
    }

    private func load() {
        guard let result = try? engine.readNode(relPath: relPath) else {
            parsed = nil
            return
        }
        parsed = result
        if result.node.projectStatus == .active {
            let env = try? engine.activationEnvironment(projectId: result.node.id)
            environment = env
            blockerNames = [:]
            for task in env?.blockedTasks ?? [] {
                if let blocker = (try? engine.blockedBy(nodeId: task.id))?.first {
                    blockerNames[task.id] = titleFor(blocker)
                }
            }
        } else {
            environment = nil
            blockerNames = [:]
        }
    }
}
