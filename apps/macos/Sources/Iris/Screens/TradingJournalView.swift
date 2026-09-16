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
            Text("+ New entry")
                .font(Typography.sans(14, weight: .medium))
                .foregroundStyle(c.textDisabled)
                .padding(.horizontal, Space.lg)
                .padding(.vertical, Space.sm)
                .background(c.bgHover)
                .clipShape(RoundedRectangle(cornerRadius: Radius.md))
                .accessibilityLabel("New trade journal entry is not available yet")
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
                stateBadge(entry)
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
            Text("Create a trading-journal-entry in the Trading Area to keep the thesis and outcomes alongside the rest of your work.")
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
            return Entry(id: cached.id, node: parsed.node, body: parsed.body.trimmingCharacters(in: .whitespacesAndNewlines))
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
        let node: FfiNode
        let body: String
    }
}
