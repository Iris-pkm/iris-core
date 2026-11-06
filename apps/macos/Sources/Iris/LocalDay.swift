import Foundation

/// User-local calendar-day strings used by task lenses and computed values.
enum LocalDay {
    static func today() -> String { string(from: Date()) }

    static func string(from date: Date) -> String { formatter.string(from: date) }

    static func date(from string: String) -> Date? { formatter.date(from: string) }

    private static let formatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.calendar = Calendar(identifier: .iso8601)
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter
    }()
}
