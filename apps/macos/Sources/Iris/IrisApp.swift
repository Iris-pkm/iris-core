import SwiftUI
import IrisCore

@main
struct IrisApp: App {
    @AppStorage("iris.appearance") private var appearance = "System"

    var body: some Scene {
        WindowGroup {
            OnboardingView()
                .preferredColorScheme(colorScheme)
        }
        .windowResizability(.contentSize)

        Settings {
            SettingsView()
                .preferredColorScheme(colorScheme)
        }
    }

    private var colorScheme: ColorScheme? {
        switch appearance {
        case "Light": return .light
        case "Dark": return .dark
        default: return nil
        }
    }
}
