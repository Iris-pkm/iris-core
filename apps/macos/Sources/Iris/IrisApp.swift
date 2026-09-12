import SwiftUI
import IrisCore

@main
struct IrisApp: App {
    var body: some Scene {
        WindowGroup {
            OnboardingView()
        }
        .windowResizability(.contentSize)
    }
}
