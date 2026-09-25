import SwiftUI
import IrisCore

/// Resource-domain aggregate over actual `reading-item` nodes.
///
/// **"+ Add" is real, added on top of the mockup** (`Reading.dc.html` has no
/// add affordance at all — this screen shipped viewer-only, same
/// undocumented gap Trading Journal at least visibly flags with a disabled
/// button). Mirrors `RemindersView`'s own new-item pattern: a small popover,
/// one `FfiEngine.createNode` call, no bespoke authoring flow.
struct ReadingListView: View {
    let engine: FfiEngine
    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }
    @State private var items: [(CachedNode, FfiNode)] = []
    @State private var filter = "All"
    @State private var showAdd = false
    @State private var draftTitle = ""
    @State private var draftURL = ""
    @State private var addError: String?
    private let filters = ["All", "Unread", "Reading", "Read"]

    var body: some View {
        HStack(spacing: 0) {
            VStack(alignment: .leading, spacing: Space.lg) {
                HStack(alignment: .firstTextBaseline) {
                    Text("Reading List").font(Typography.h1()).foregroundStyle(c.textPrimary)
                    Spacer()
                    Button("+ Add") { showAdd = true }
                        .buttonStyle(.bordered)
                        .popover(isPresented: $showAdd) { addPopover }
                }
                Text("\(shown.count) item\(shown.count == 1 ? "" : "s")").font(Typography.bodySans()).foregroundStyle(c.textSecondary)
                HStack(spacing: Space.sm) { ForEach(filters, id: \.self) { chip($0) } }
                ForEach(shown, id: \.0.id) { item in row(item) }
                Spacer()
            }
            .padding(EdgeInsets(top: Space.xxl, leading: Space.xxxl, bottom: Space.xl, trailing: Space.xxxl))
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            Divider().background(c.borderDefault)
            rail
        }.background(c.bgCanvas).task { load() }
    }

    private var addPopover: some View {
        VStack(alignment: .leading, spacing: Space.sm) {
            Text("Add reading item").font(Typography.sans(14, weight: .medium)).foregroundStyle(c.textPrimary)
            TextField("Title", text: $draftTitle).textFieldStyle(.roundedBorder)
            TextField("Source URL (optional)", text: $draftURL).textFieldStyle(.roundedBorder)
            if let addError {
                Text(addError).font(Typography.caption()).foregroundStyle(c.danger)
            }
            HStack {
                Spacer()
                Button("Cancel") { resetDraft() }
                Button("Add") { addItem() }
                    .buttonStyle(.borderedProminent)
                    .disabled(draftTitle.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }
        .padding(Space.lg)
        .frame(width: 320)
    }

    private func resetDraft() {
        draftTitle = ""
        draftURL = ""
        addError = nil
        showAdd = false
    }

    private func addItem() {
        let title = draftTitle.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !title.isEmpty else { return }
        let url = draftURL.trimmingCharacters(in: .whitespacesAndNewlines)
        let id = newNodeId()
        let now = ISO8601DateFormatter().string(from: Date())
        let node = FfiNode(
            id: id, nodeType: .readingItem, created: now, modified: now, schemaVersion: currentSchemaVersion(),
            lifecycle: nil, archivedAt: nil, domain: nil, tags: [], relations: [], deletedAt: nil, isTemplate: false,
            distillationLevel: nil, status: nil, priority: nil, scheduledDate: nil, dueDate: nil, estimatedPomodoros: nil,
            actualPomodoros: nil, recurrence: nil, recurrenceOccurrences: nil, checklist: [], start: nil, end: nil,
            externalId: nil, projectStatus: nil, startDate: nil, targetDate: nil, sourceUrl: url.isEmpty ? nil : url,
            readStatus: "unread", reminderText: nil, fireAt: nil, reminderStatus: nil, resolved: false, anchor: nil,
            pinned: [], activeFilter: nil, defaultView: nil, theme: nil, inkAttachment: nil, date: nil, symbol: nil,
            entry: nil, exit: nil, pnl: nil, rMultiple: nil
        )
        let slug = title.lowercased()
            .replacingOccurrences(of: "[^a-z0-9]+", with: "-", options: .regularExpression)
            .trimmingCharacters(in: CharacterSet(charactersIn: "-"))
        let path = "reading/\(slug.isEmpty ? id.lowercased() : slug).md"
        do {
            try engine.createNode(relPath: path, node: node, body: "\n\(title)\n")
            resetDraft()
            load()
        } catch {
            addError = String(describing: error)
        }
    }
    private var shown: [(CachedNode, FfiNode)] { filter == "All" ? items : items.filter { ($0.1.readStatus ?? "Unread").capitalized == filter } }
    private func chip(_ name: String) -> some View { Button(name) { filter = name }.buttonStyle(.plain).font(Typography.caption()).foregroundStyle(filter == name ? c.accentDefault : c.textSecondary).padding(.horizontal, Space.md).padding(.vertical, Space.xs).background(filter == name ? c.bgSelected : c.bgHover).clipShape(Capsule()) }
    private func row(_ item: (CachedNode, FfiNode)) -> some View {
        HStack(spacing: Space.md) {
            Image(systemName: item.1.sourceUrl == nil ? "book" : "doc.text").foregroundStyle(c.textSecondary).frame(width: 14)
            VStack(alignment: .leading, spacing: Space.xxs) {
                Text(title(item.0)).font(Typography.bodySans()).foregroundStyle(c.textPrimary)
                Text(item.1.sourceUrl ?? "Internal note").font(Typography.mono(11)).foregroundStyle(c.textSecondary)
            }; Spacer(); status(item.1.readStatus ?? "Unread")
        }.padding(.vertical, Space.sm)
    }
    private var rail: some View {
        VStack(alignment: .leading, spacing: Space.lg) {
            Text("UP NEXT").font(Typography.caption()).foregroundStyle(c.textSecondary)
            ForEach(Array(items.filter { ($0.1.readStatus ?? "unread").lowercased() != "read" }.prefix(3)), id: \.0.id) { Text(title($0.0)).font(Typography.bodySmall()).foregroundStyle(c.textPrimary) }
            Divider().background(c.borderDefault)
            Text("STATUS").font(Typography.caption()).foregroundStyle(c.textSecondary)
            ForEach(["Unread", "Reading", "Read"], id: \.self) { name in HStack { Text(name).font(Typography.bodySmall()).foregroundStyle(c.textPrimary); Spacer(); Text("\(items.filter { ($0.1.readStatus ?? "Unread").capitalized == name }.count)").font(Typography.bodySmall()).foregroundStyle(c.textSecondary) } }
            Spacer()
        }.padding(Space.lg).frame(width: 260).background(c.bgSurface)
    }
    private func status(_ value: String) -> some View { Text(value.capitalized).font(Typography.caption()).foregroundStyle(c.textSecondary).padding(.horizontal, Space.sm).padding(.vertical, Space.xxs).background(c.bgHover).clipShape(Capsule()) }
    private func title(_ node: CachedNode) -> String { (node.path as NSString).lastPathComponent.replacingOccurrences(of: ".md", with: "") }
    private func load() { let nodes = (try? engine.search(query: "", nodeType: "reading-item", domain: nil, tag: nil)) ?? []; items = nodes.compactMap { n in (try? engine.readNode(relPath: n.path)).map { (n, $0.node) } } }
}
