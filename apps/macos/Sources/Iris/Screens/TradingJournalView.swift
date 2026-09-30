import Foundation
import SwiftUI
import IrisCore

/// The Trading Area's journal subpage. Trade facts come directly from
/// `trading-journal-entry` nodes (ADR-033); the weekly figures are a small
/// derived presentation, not a second source of truth.
struct TradingJournalView: View {
    let engine: FfiEngine

    @Environment(\.colorScheme) private var colorScheme
    private var c: Palette.Colors { Palette.colors(for: colorScheme) }
    @State private var entries: [Entry] = []
    @State private var showAdd = false
    @State private var draftSymbol = ""
    @State private var draftEntry = ""
    @State private var draftExit = ""
    @State private var draftPNL = ""
    @State private var draftThesis = ""
    @State private var addError: String?
    @State private var editingPath: String?
    @State private var editSymbol = ""
    @State private var editEntry = ""
    @State private var editExit = ""
    @State private var editPNL = ""

    var body: some View {
        HStack(spacing: 0) {
            ScrollView {
                VStack(alignment: .leading, spacing: Space.xl) {
                    header

                    if entries.isEmpty {
                        emptyState
                    } else {
                        LazyVStack(spacing: Space.md) {
                            ForEach(entries) { entry in
                                card(entry)
                            }
                        }
                    }
                }
                .padding(EdgeInsets(top: Space.xxl, leading: Space.xxxl, bottom: Space.xxl, trailing: Space.xxxl))
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)

            Divider().background(c.borderDefault)
            rail
        }
        .background(c.bgCanvas)
        .task { load() }
    }

    private var header: some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: Space.xs) {
                Text("Trading Journal")
                    .font(Typography.h1())
                    .foregroundStyle(c.textPrimary)
                Text("Trading / Journal · \(weekEntries.count) entr\(weekEntries.count == 1 ? "y" : "ies") this week")
                    .font(Typography.bodySans())
                    .foregroundStyle(c.textSecondary)
            }
            Spacer()
            Button("+ New entry") { showAdd = true }
                .buttonStyle(.bordered)
                .popover(isPresented: $showAdd) { addPopover }
        }
    }

    private var addPopover: some View {
        VStack(alignment: .leading, spacing: Space.sm) {
            Text("New trade entry").font(Typography.sans(14, weight: .medium)).foregroundStyle(c.textPrimary)
            TextField("Symbol", text: $draftSymbol).textFieldStyle(.roundedBorder)
            TextField("Entry price", text: $draftEntry).textFieldStyle(.roundedBorder)
            TextField("Exit price (blank = open)", text: $draftExit).textFieldStyle(.roundedBorder)
            TextField("P&L (optional)", text: $draftPNL).textFieldStyle(.roundedBorder)
            TextField("Thesis", text: $draftThesis, axis: .vertical).lineLimit(3...6).textFieldStyle(.roundedBorder)
            if let addError {
                Text(addError).font(Typography.caption()).foregroundStyle(c.danger)
            }
            HStack {
                Spacer()
                Button("Cancel") { resetDraft() }
                Button("Add") { addEntry() }
                    .buttonStyle(.borderedProminent)
                    .disabled(draftSymbol.trimmingCharacters(in: .whitespaces).isEmpty || Double(draftEntry) == nil)
            }
        }
        .padding(Space.lg)
        .frame(width: 320)
    }

    private func resetDraft() {
        draftSymbol = ""; draftEntry = ""; draftExit = ""; draftPNL = ""; draftThesis = ""
        addError = nil
        showAdd = false
    }

    private func addEntry() {
        let symbol = draftSymbol.trimmingCharacters(in: .whitespaces).uppercased()
        guard !symbol.isEmpty, let entryPrice = Double(draftEntry) else { return }
        let exit = Double(draftExit)
        let id = newNodeId()
        let nowDate = Date()
        let now = ISO8601DateFormatter().string(from: nowDate)
        let node = FfiNode(
            id: id, nodeType: .tradingJournalEntry, created: now, modified: now, schemaVersion: currentSchemaVersion(),
            lifecycle: nil, archivedAt: nil, domain: "trading", tags: [], relations: [], deletedAt: nil, isTemplate: false,
            distillationLevel: nil, status: nil, priority: nil, scheduledDate: nil, dueDate: nil, estimatedPomodoros: nil,
            actualPomodoros: nil, recurrence: nil, recurrenceOccurrences: nil, checklist: [], start: nil, end: nil,
            externalId: nil, projectStatus: nil, startDate: nil, targetDate: nil, sourceUrl: nil,
            readStatus: nil, reminderText: nil, fireAt: nil, reminderStatus: nil, resolved: false, anchor: nil,
            pinned: [], activeFilter: nil, defaultView: nil, theme: nil, inkAttachment: nil, date: nil, symbol: symbol,
            entry: entryPrice, exit: exit, pnl: Double(draftPNL), rMultiple: nil
        )
        let stamp = nowDate.formatted(.iso8601.year().month().day().dateSeparator(.dash)) // yyyy-MM-dd
        let slug = symbol.lowercased().replacingOccurrences(of: "[^a-z0-9]+", with: "-", options: .regularExpression)
        let path = "trading/journal/\(stamp)-\(slug)-\(id.suffix(6).lowercased()).md"
        do {
            try engine.createNode(relPath: path, node: node, body: "\n\(draftThesis.trimmingCharacters(in: .whitespacesAndNewlines))\n")
            resetDraft()
            load()
        } catch {
            addError = String(describing: error)
        }
    }

    private func card(_ entry: Entry) -> some View {
        VStack(alignment: .leading, spacing: Space.md) {
            HStack(alignment: .firstTextBaseline) {
                VStack(alignment: .leading, spacing: Space.xxs) {
                    Text(entry.node.symbol ?? "Untitled trade")
                        .font(Typography.sans(15, weight: .semibold))
                        .foregroundStyle(c.textPrimary)
                    Text(displayDate(entry.node.created))
                        .font(Typography.bodySmall())
                        .foregroundStyle(c.textSecondary)
                }
                Spacer()
                Button { beginEdit(entry) } label: { Image(systemName: "pencil") }
                    .buttonStyle(.plain).foregroundStyle(c.textSecondary).help("Edit trade")
                    .accessibilityLabel("Edit trade")
                    .popover(isPresented: Binding(get: { editingPath == entry.path }, set: { if !$0 { editingPath = nil } })) { editPopover(entry.path) }
                statusMenu(entry)
            }

            Text(priceLine(entry))
                .font(Typography.bodySmall())
                .foregroundStyle(c.textSecondary)

            if !entry.body.isEmpty {
                Text(entry.body)
                    .font(Typography.serif(16))
                    .italic()
                    .foregroundStyle(c.textPrimary)
                    .lineSpacing(4)
                    .fixedSize(horizontal: false, vertical: true)
            }

            HStack {
                if let rMultiple = entry.node.rMultiple {
                    Text(String(format: "%.1fR", rMultiple))
                        .font(Typography.caption())
                        .foregroundStyle(c.textSecondary)
                }
                Spacer()
                if let pnl = entry.node.pnl {
                    pnlBadge(pnl, isOpen: entry.node.exit == nil)
                }
            }
        }
        .padding(Space.lg)
        .background(c.bgSurface)
        .overlay(RoundedRectangle(cornerRadius: Radius.lg).stroke(c.borderDefault, lineWidth: 1))
        .clipShape(RoundedRectangle(cornerRadius: Radius.lg))
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(entry.node.symbol ?? "Trade"), \(priceLine(entry))")
    }

    private func statusMenu(_ entry: Entry) -> some View {
        Menu {
            if entry.node.exit == nil {
                Button("Close trade…") { beginEdit(entry) }
            } else {
                Button("Reopen (clears exit and P&L)") { update(entry.path) { $0.exit = nil; $0.pnl = nil } }
            }
        } label: { stateBadge(entry) }
            .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
            .accessibilityLabel("Trade is \(entry.node.exit == nil ? "open" : "closed"), change status")
    }

    private func beginEdit(_ entry: Entry) {
        editSymbol = entry.node.symbol ?? ""
        editEntry = entry.node.entry.map { String($0) } ?? ""
        editExit = entry.node.exit.map { String($0) } ?? ""
        editPNL = entry.node.pnl.map { String($0) } ?? ""
        editingPath = entry.path
    }

    private func editPopover(_ path: String) -> some View {
        VStack(alignment: .leading, spacing: Space.sm) {
            Text("Edit trade").font(Typography.sans(14, weight: .medium)).foregroundStyle(c.textPrimary)
            TextField("Symbol", text: $editSymbol).textFieldStyle(.roundedBorder)
            TextField("Entry price", text: $editEntry).textFieldStyle(.roundedBorder)
            TextField("Exit price (blank = open)", text: $editExit).textFieldStyle(.roundedBorder)
            TextField("P&L (blank to clear)", text: $editPNL).textFieldStyle(.roundedBorder)
            HStack {
                Spacer()
                Button("Cancel") { editingPath = nil }
                Button("Save") {
                    let symbol = editSymbol.trimmingCharacters(in: .whitespaces).uppercased()
                    update(path) {
                        $0.symbol = symbol.isEmpty ? nil : symbol
                        $0.entry = Double(editEntry)
                        $0.exit = Double(editExit)
                        $0.pnl = Double(editPNL)
                    }
                    editingPath = nil
                }
                .buttonStyle(.borderedProminent)
                .disabled(editSymbol.trimmingCharacters(in: .whitespaces).isEmpty || Double(editEntry) == nil)
            }
        }
        .padding(Space.lg)
        .frame(width: 320)
    }

    /// Read-modify-write via `updateNode` (thesis body is untouched).
    private func update(_ path: String, _ change: (inout FfiNode) -> Void) {
        guard let parsed = try? engine.readNode(relPath: path) else { return }
        var node = parsed.node
        change(&node)
        node.modified = ISO8601DateFormatter().string(from: Date())
        _ = try? engine.updateNode(relPath: path, node: node)
        load()
    }

    private func stateBadge(_ entry: Entry) -> some View {
        Text(entry.node.exit == nil ? "OPEN" : "CLOSED")
            .font(Typography.caption())
            .foregroundStyle(entry.node.exit == nil ? c.warning : c.textSecondary)
            .padding(.horizontal, Space.sm)
            .padding(.vertical, Space.xxs)
            .background(entry.node.exit == nil ? c.warningTint : c.bgHover)
            .clipShape(Capsule())
    }

    private func pnlBadge(_ pnl: Double, isOpen: Bool) -> some View {
        let positive = pnl >= 0
        return Text("\(isOpen ? "Unrealized " : "")\(money(pnl))")
            .font(Typography.caption())
            .foregroundStyle(positive ? c.success : c.danger)
            .padding(.horizontal, Space.sm)
            .padding(.vertical, Space.xxs)
            .background(positive ? c.successTint : c.dangerTint)
            .clipShape(Capsule())
    }

    private var rail: some View {
        VStack(alignment: .leading, spacing: Space.lg) {
            Text("THIS WEEK")
                .font(Typography.caption())
                .foregroundStyle(c.textSecondary)
            metric("Win rate", winRate)
            metric("Total P&L", totalPNL.map(money) ?? "—", emphasis: totalPNL)
            metric("Average R-multiple", averageR.map { String(format: "%.1fR", $0) } ?? "—")
            metric("Best trade", bestTrade ?? "—")

            Divider().background(c.borderDefault)

            Text("DISTILLATION")
                .font(Typography.caption())
                .foregroundStyle(c.textSecondary)
            Text("\(undistilledCount) entr\(undistilledCount == 1 ? "y" : "ies") not reviewed")
                .font(Typography.bodySmall())
                .foregroundStyle(c.textPrimary)
            Text("Weekly retro is available once you have a complete week of entries.")
                .font(Typography.bodySmall())
                .foregroundStyle(c.textSecondary)
                .fixedSize(horizontal: false, vertical: true)
            Spacer()
        }
        .padding(Space.lg)
        .frame(width: 264, alignment: .topLeading)
        .background(c.bgSurface)
    }

    private func metric(_ label: String, _ value: String, emphasis: Double? = nil) -> some View {
        HStack(alignment: .firstTextBaseline) {
            Text(label).font(Typography.bodySmall()).foregroundStyle(c.textSecondary)
            Spacer()
            Text(value)
                .font(Typography.bodySmall())
                .foregroundStyle(emphasis.map { $0 >= 0 ? c.success : c.danger } ?? c.textPrimary)
        }
    }

    private var emptyState: some View {
        VStack(alignment: .leading, spacing: Space.sm) {
            Text("Your trade log starts here.")
                .font(Typography.sans(15, weight: .semibold))
                .foregroundStyle(c.textPrimary)
            Text("Use “+ New entry” to log a trade and keep the thesis and outcomes alongside the rest of your work.")
                .font(Typography.bodySans())
                .foregroundStyle(c.textSecondary)
                .frame(maxWidth: 420, alignment: .leading)
        }
        .padding(.top, Space.xxxl)
    }

    private var weekEntries: [Entry] {
        let calendar = Calendar.current
        guard let interval = calendar.dateInterval(of: .weekOfYear, for: Date()) else { return [] }
        return entries.filter { entry in
            parsedDate(entry.node.created).map(interval.contains) ?? false
        }
    }

    private var closedWeekEntries: [Entry] { weekEntries.filter { $0.node.exit != nil } }
    private var winRate: String {
        guard !closedWeekEntries.isEmpty else { return "—" }
        let wins = closedWeekEntries.filter { ($0.node.pnl ?? 0) > 0 }.count
        return "\(Int((Double(wins) / Double(closedWeekEntries.count) * 100).rounded()))%"
    }
    private var totalPNL: Double? {
        let values = closedWeekEntries.compactMap(\.node.pnl)
        return values.isEmpty ? nil : values.reduce(0, +)
    }
    private var averageR: Double? {
        let values = closedWeekEntries.compactMap(\.node.rMultiple)
        return values.isEmpty ? nil : values.reduce(0, +) / Double(values.count)
    }
    private var bestTrade: String? {
        closedWeekEntries.compactMap { entry in
            entry.node.pnl.map { (entry.node.symbol ?? "Trade", $0) }
        }.max(by: { $0.1 < $1.1 }).map { "\($0.0) \(money($0.1))" }
    }
    private var undistilledCount: Int {
        entries.filter { $0.node.distillationLevel == nil }.count
    }

    private func load() {
        let nodes = (try? engine.search(query: "", nodeType: "trading-journal-entry", domain: nil, tag: nil)) ?? []
        entries = nodes.compactMap { cached in
            guard let parsed = try? engine.readNode(relPath: cached.path) else { return nil }
            return Entry(id: cached.id, path: cached.path, node: parsed.node, body: parsed.body.trimmingCharacters(in: .whitespacesAndNewlines))
        }.sorted { $0.node.created > $1.node.created }
    }

    private func priceLine(_ entry: Entry) -> String {
        guard let entryPrice = entry.node.entry else { return "Entry not recorded" }
        if let exit = entry.node.exit {
            return "Entry \(price(entryPrice)) · Exit \(price(exit))"
        }
        return "Entry \(price(entryPrice)) · Position open"
    }
    private func price(_ value: Double) -> String { String(format: "$%.2f", value) }
    private func money(_ value: Double) -> String { String(format: "%@ $%.2f", value >= 0 ? "+" : "−", abs(value)) }
    private func displayDate(_ value: String) -> String {
        guard let date = parsedDate(value) else { return value }
        return date.formatted(.dateTime.month(.abbreviated).day())
    }
    private func parsedDate(_ value: String) -> Date? {
        ISO8601DateFormatter().date(from: value)
    }

    private struct Entry: Identifiable {
        let id: String
        let path: String
        let node: FfiNode
        let body: String
    }
}
