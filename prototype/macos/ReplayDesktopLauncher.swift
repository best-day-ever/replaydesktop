// ReplayDesktop technical launcher
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This launcher deliberately remains a thin, inspectable wrapper around Kyber's
// kyclient executable. It does not alter the media path or claim capabilities
// that the child has not proved at runtime.

import AppKit
import Darwin
import Foundation

private struct LauncherError: LocalizedError {
    let message: String

    var errorDescription: String? {
        message
    }
}

private enum CodecMode: String, CaseIterable {
    case h264420
    case hevc420
    case hevc444
    case av1420

    var title: String {
        switch self {
        case .h264420:
            return "H.264 4:2:0 — proven baseline"
        case .hevc420:
            return "HEVC 4:2:0 — experimental/manual"
        case .hevc444:
            return "HEVC 4:4:4 — experimental/manual"
        case .av1420:
            return "AV1 4:2:0 — experimental/manual"
        }
    }

    var codecArgument: String {
        switch self {
        case .h264420:
            return "h264"
        case .hevc420, .hevc444:
            return "hevc"
        case .av1420:
            return "av1"
        }
    }

    var chroma: String {
        switch self {
        case .hevc444:
            return "4:4:4"
        case .h264420, .hevc420, .av1420:
            return "4:2:0"
        }
    }

    var usesYUV444: Bool {
        self == .hevc444
    }

    var qualification: String {
        switch self {
        case .h264420:
            return "Proven launcher baseline; actual hardware path is still reported by Kyber."
        case .hevc420:
            return "Manual until the exact host encoder and client decoder path succeeds."
        case .hevc444:
            return "Manual fidelity mode; hardware/software decode must be verified from telemetry."
        case .av1420:
            return "Manual; requires an Ada-or-newer host and a proven client decode path."
        }
    }

    static func parse(codec: String, chroma: String) throws -> CodecMode {
        let normalizedCodec = codec.lowercased()
        let normalizedChroma = chroma.replacingOccurrences(of: ":", with: "")

        switch (normalizedCodec, normalizedChroma) {
        case ("h264", "420"), ("h.264", "420"):
            return .h264420
        case ("hevc", "420"), ("h265", "420"), ("h.265", "420"):
            return .hevc420
        case ("hevc", "444"), ("h265", "444"), ("h.265", "444"):
            return .hevc444
        case ("av1", "420"):
            return .av1420
        case ("av1", "444"):
            throw LauncherError(message: "AV1 4:4:4 is unsupported and will never be passed to kyclient.")
        default:
            throw LauncherError(
                message: "Unsupported codec/chroma pair '\(codec) \(chroma)'."
            )
        }
    }
}

private enum DisplayMode: String, CaseIterable {
    case single
    case multiple

    var title: String {
        switch self {
        case .single:
            return "Single physical display"
        case .multiple:
            return "Multiple physical displays"
        }
    }
}

private enum KymuxAudioTransport: String, CaseIterable {
    case reliable
    case unreliable
    case unreliableFEC = "unreliable_fec"

    var title: String {
        switch self {
        case .reliable:
            return "Reliable"
        case .unreliable:
            return "Unreliable"
        case .unreliableFEC:
            return "Unreliable + FEC"
        }
    }
}

private enum MacOSInputPolicy {
    static let immersiveSystemShortcutSuppressionAvailable = false
    static let keyboardGrabArgument = "--keyboard-grab=false"
    static let inputControlTitle = "Forward mouse + focused-window keyboard input"
    static let immersiveStatus = "Unavailable on macOS"
}

private struct LauncherConfiguration {
    var host = ""
    var port = 8080
    var codec = CodecMode.h264420
    var bitrate = "100M"
    var displayMode = DisplayMode.single
    var displayValue = 0
    var audioEnabled = false
    var audioBufferMilliseconds = 40
    var audioTransport = KymuxAudioTransport.reliable
    var inputsEnabled = true

    func validated() throws -> LauncherConfiguration {
        var result = self
        result.host = host.trimmingCharacters(in: .whitespacesAndNewlines)
        result.bitrate = bitrate.trimmingCharacters(in: .whitespacesAndNewlines)

        guard !result.host.isEmpty else {
            throw LauncherError(message: "Enter a directly reachable host name or IP address.")
        }
        guard !result.host.hasPrefix("-") else {
            throw LauncherError(
                message: "Host names and IP addresses cannot begin with '-'."
            )
        }
        guard result.host.unicodeScalars.allSatisfy({
            !CharacterSet.whitespacesAndNewlines.contains($0)
                && !CharacterSet.controlCharacters.contains($0)
        }) else {
            throw LauncherError(message: "Host names and IP addresses cannot contain whitespace.")
        }
        guard (1 ... 65_535).contains(result.port) else {
            throw LauncherError(message: "Port must be between 1 and 65535.")
        }
        _ = try validatedBitrate(result.bitrate)

        switch result.displayMode {
        case .single:
            guard (0 ... 15).contains(result.displayValue) else {
                throw LauncherError(message: "Single-display index must be between 0 and 15.")
            }
        case .multiple:
            guard (2 ... 8).contains(result.displayValue) else {
                throw LauncherError(message: "Multi-monitor count must be between 2 and 8.")
            }
        }

        guard (0 ... 2_000).contains(result.audioBufferMilliseconds) else {
            throw LauncherError(message: "Audio buffer must be between 0 and 2000 ms.")
        }

        return result
    }
}

private func validatedBitrate(_ input: String) throws -> UInt32 {
    guard !input.isEmpty else {
        throw LauncherError(message: "Total video bitrate is required.")
    }

    var digits = input
    var scale: UInt64 = 1
    if let last = input.last, last == "M" || last == "K" {
        digits.removeLast()
        scale = last == "M" ? 1_000_000 : 1_000
    }

    guard !digits.isEmpty,
          digits.allSatisfy(\.isNumber),
          let value = UInt64(digits),
          value > 0,
          value <= UInt64(UInt32.max) / scale
    else {
        throw LauncherError(
            message: "Bitrate must be a positive integer, optionally followed by uppercase K or M."
        )
    }

    return UInt32(value * scale)
}

private enum KyclientArguments {
    // Keep this exact sequence visible. The selected port replaces only the
    // first value, avoiding a duplicate Clap option while preserving the proven
    // default baseline byte-for-byte.
    static let fixedBaseline = [
        "--port=8080",
        "--protocol=kymux",
        "--tls-skip-verification",
        "--video-buffer=0",
        "--metrics=true",
        "--auto-reconnect=false",
    ]

    static func build(for configuration: LauncherConfiguration) throws -> [String] {
        let configuration = try configuration.validated()
        var arguments = fixedBaseline
        arguments[0] = "--port=\(configuration.port)"

        arguments.append("--video-codec=\(configuration.codec.codecArgument)")
        if configuration.codec.usesYUV444 {
            arguments.append("--444")
        }
        arguments.append("--bitrate=\(configuration.bitrate)")

        switch configuration.displayMode {
        case .single:
            arguments.append("--display-idx=\(configuration.displayValue)")
        case .multiple:
            // Kymux is already forced by the fixed baseline.
            arguments.append("--display-count=\(configuration.displayValue)")
        }

        arguments.append("--audio=\(configuration.audioEnabled)")
        arguments.append("--audio-buffer=\(configuration.audioBufferMilliseconds)")
        arguments.append("--kymux-audio=\(configuration.audioTransport.rawValue)")
        arguments.append("--inputs=\(configuration.inputsEnabled)")
        arguments.append(MacOSInputPolicy.keyboardGrabArgument)

        // Clipboard is intentionally omitted: this macOS kyclient build does
        // not expose a working clipboard pipeline.
        arguments.append("--")
        arguments.append(configuration.host)
        return arguments
    }
}

private enum LauncherPaths {
    static var childExecutable: URL {
        let bundleURL = Bundle.main.bundleURL
        if bundleURL.pathExtension.lowercased() == "app" {
            return bundleURL
                .appendingPathComponent("Contents", isDirectory: true)
                .appendingPathComponent("MacOS", isDirectory: true)
                .appendingPathComponent("kyclient", isDirectory: false)
        }

        return URL(fileURLWithPath: CommandLine.arguments[0])
            .standardizedFileURL
            .deletingLastPathComponent()
            .appendingPathComponent("kyclient", isDirectory: false)
    }

    static var applicationSupportDirectory: URL {
        let base = FileManager.default.urls(
            for: .applicationSupportDirectory,
            in: .userDomainMask
        ).first ?? FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Application Support", isDirectory: true)
        return base.appendingPathComponent("ReplayDesktop", isDirectory: true)
    }

    static func createApplicationSupportDirectory() throws -> URL {
        let directory = applicationSupportDirectory
        try FileManager.default.createDirectory(
            at: directory,
            withIntermediateDirectories: true,
            attributes: nil
        )
        guard FileManager.default.isWritableFile(atPath: directory.path) else {
            throw LauncherError(
                message: "Runtime directory is not writable: \(directory.path)"
            )
        }
        return directory
    }
}

private func shellQuoted(_ argument: String) -> String {
    let safe = CharacterSet(
        charactersIn: "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_./:=+-"
    )
    if argument.unicodeScalars.allSatisfy({ safe.contains($0) }) {
        return argument
    }
    return "'" + argument.replacingOccurrences(of: "'", with: "'\\''") + "'"
}

private func commandDescription(executable: URL, arguments: [String]) -> String {
    ([executable.path] + arguments).map(shellQuoted).joined(separator: " ")
}

private enum PreferenceKey {
    static let host = "prototype.host"
    static let port = "prototype.port"
    static let codec = "prototype.codec"
    static let bitrate = "prototype.bitrate"
    static let displayMode = "prototype.displayMode"
    static let displayValue = "prototype.displayValue"
    static let audioEnabled = "prototype.audioEnabled"
    static let audioBuffer = "prototype.audioBuffer"
    static let audioTransport = "prototype.audioTransport"
    static let inputsEnabled = "prototype.inputsEnabled"
}

private struct TailCursor {
    var initialized = false
    var offset: UInt64 = 0
}

@MainActor
private final class LauncherController: NSObject, NSTextFieldDelegate, NSWindowDelegate {
    private let window: NSWindow
    private let hostField = NSTextField()
    private let portField = NSTextField()
    private let codecPopup = NSPopUpButton(frame: .zero, pullsDown: false)
    private let chromaValue = NSTextField(labelWithString: "4:2:0")
    private let codecQualification = NSTextField(wrappingLabelWithString: "")
    private let bitrateField = NSTextField()
    private let displayModePopup = NSPopUpButton(frame: .zero, pullsDown: false)
    private let displayValueLabel = NSTextField(labelWithString: "Display index")
    private let displayValueField = NSTextField()
    private let audioCheckbox = NSButton(
        checkboxWithTitle: "Request the existing Kyber audio stream",
        target: nil,
        action: nil
    )
    private let audioBufferField = NSTextField()
    private let audioTransportPopup = NSPopUpButton(frame: .zero, pullsDown: false)
    private let inputsCheckbox = NSButton(
        checkboxWithTitle: MacOSInputPolicy.inputControlTitle,
        target: nil,
        action: nil
    )
    private let immersiveInputStatus = NSTextField(
        wrappingLabelWithString: MacOSInputPolicy.immersiveStatus
    )
    private let validationLabel = NSTextField(wrappingLabelWithString: "")
    private let commandPreview = NSTextField(wrappingLabelWithString: "")
    private let sessionStatus = NSTextField(labelWithString: "Disconnected")
    private let connectButton = NSButton(title: "Connect", target: nil, action: nil)
    private let disconnectButton = NSButton(title: "Disconnect", target: nil, action: nil)
    private let telemetryView = NSTextView()

    private var child: Process?
    private var stdoutPipe: Pipe?
    private var stderrPipe: Pipe?
    private var telemetryTimer: Timer?
    private var tailCursors: [URL: TailCursor] = [:]
    private var transcript = ""
    private var disconnectRequested = false

    private let maximumTranscriptCharacters = 120_000
    private let initialTailBytes: UInt64 = 32 * 1_024
    private let maximumTailReadBytes: UInt64 = 64 * 1_024

    override init() {
        window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 1_000, height: 900),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )
        super.init()

        configureControls()
        buildInterface()
        loadPreferences()
        updateDerivedControls()
        updateCommandPreview()

        window.title = "ReplayDesktop — Technical Launcher"
        window.minSize = NSSize(width: 880, height: 760)
        window.center()
        window.delegate = self
    }

    func show() {
        window.makeKeyAndOrderFront(nil)
        NSApp.activate()
    }

    func persistCurrentPreferences() {
        if let configuration = try? configurationFromControls() {
            savePreferences(configuration)
        }
    }

    func terminateChildForApplicationExit() {
        telemetryTimer?.invalidate()
        telemetryTimer = nil
        if let child, child.isRunning {
            child.terminate()
        }
    }

    private func configureControls() {
        hostField.placeholderString = "Direct LAN/VPN hostname or IP"
        hostField.delegate = self
        hostField.setAccessibilityLabel("Linux host name or IP address")

        portField.delegate = self
        portField.alignment = .right
        portField.setAccessibilityLabel("Kyber port")

        codecPopup.addItems(withTitles: CodecMode.allCases.map(\.title))
        codecPopup.target = self
        codecPopup.action = #selector(controlChanged(_:))

        chromaValue.font = .monospacedSystemFont(ofSize: NSFont.systemFontSize, weight: .medium)
        codecQualification.textColor = .secondaryLabelColor

        bitrateField.delegate = self
        bitrateField.placeholderString = "100M"
        bitrateField.setAccessibilityLabel("Total video bitrate")

        displayModePopup.addItems(withTitles: DisplayMode.allCases.map(\.title))
        displayModePopup.target = self
        displayModePopup.action = #selector(controlChanged(_:))

        displayValueField.delegate = self
        displayValueField.alignment = .right
        displayValueField.setAccessibilityLabel("Display index or display count")

        audioCheckbox.target = self
        audioCheckbox.action = #selector(controlChanged(_:))
        audioBufferField.delegate = self
        audioBufferField.alignment = .right
        audioBufferField.setAccessibilityLabel("Audio buffer in milliseconds")

        audioTransportPopup.addItems(withTitles: KymuxAudioTransport.allCases.map(\.title))
        audioTransportPopup.target = self
        audioTransportPopup.action = #selector(controlChanged(_:))

        inputsCheckbox.target = self
        inputsCheckbox.action = #selector(controlChanged(_:))
        immersiveInputStatus.textColor = .systemOrange
        immersiveInputStatus.font =
            .systemFont(ofSize: NSFont.systemFontSize, weight: .semibold)

        validationLabel.textColor = .systemRed
        commandPreview.font = .monospacedSystemFont(ofSize: 11, weight: .regular)
        commandPreview.textColor = .secondaryLabelColor
        commandPreview.isSelectable = true
        commandPreview.lineBreakMode = .byCharWrapping

        sessionStatus.font = .systemFont(ofSize: NSFont.systemFontSize, weight: .semibold)
        connectButton.bezelStyle = .rounded
        connectButton.keyEquivalent = "\r"
        connectButton.target = self
        connectButton.action = #selector(connectClicked(_:))

        disconnectButton.bezelStyle = .rounded
        disconnectButton.target = self
        disconnectButton.action = #selector(disconnectClicked(_:))
        disconnectButton.isEnabled = false

        telemetryView.isEditable = false
        telemetryView.isSelectable = true
        telemetryView.font = .monospacedSystemFont(ofSize: 11, weight: .regular)
        telemetryView.textContainerInset = NSSize(width: 8, height: 8)
        telemetryView.backgroundColor = .textBackgroundColor
        telemetryView.string = "No session output yet.\n"
    }

    private func buildInterface() {
        let content = NSView()
        window.contentView = content

        let root = NSStackView()
        root.orientation = .vertical
        root.alignment = .leading
        root.spacing = 10
        root.translatesAutoresizingMaskIntoConstraints = false
        content.addSubview(root)

        NSLayoutConstraint.activate([
            root.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 20),
            root.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -20),
            root.topAnchor.constraint(equalTo: content.topAnchor, constant: 18),
            root.bottomAnchor.constraint(equalTo: content.bottomAnchor, constant: -18),
        ])

        let title = NSTextField(labelWithString: "Direct Kyber connection")
        title.font = .systemFont(ofSize: 22, weight: .semibold)
        root.addArrangedSubview(title)

        let subtitle = NSTextField(
            wrappingLabelWithString:
                "Technical prototype panel. It starts the unchanged Contents/MacOS/kyclient child; "
                + "the raw child remains available for direct CLI use."
        )
        subtitle.textColor = .secondaryLabelColor
        root.addArrangedSubview(subtitle)
        constrainWidth(subtitle, to: root)

        let trustWarning = NSTextField(
            wrappingLabelWithString:
                "Internal LAN/VPN prototype: TLS identity verification is intentionally bypassed. "
                + "No CA, account, JWT, username, or password setup is provided here."
        )
        trustWarning.textColor = .systemOrange
        trustWarning.font = .systemFont(ofSize: NSFont.systemFontSize, weight: .medium)
        root.addArrangedSubview(trustWarning)
        constrainWidth(trustWarning, to: root)

        root.addArrangedSubview(sectionTitle("Connection and video"))
        addRow(
            to: root,
            label: "Linux host / IP",
            control: hostField,
            detail: "Directly reachable host only; no discovery or relay."
        )
        addRow(
            to: root,
            label: "Port",
            control: portField,
            detail: "Default 8080. Replaces the baseline port slot."
        )
        addRow(
            to: root,
            label: "Codec + chroma",
            control: codecPopup,
            detailView: codecQualification
        )
        addRow(
            to: root,
            label: "Selected chroma",
            control: chromaValue,
            detail: "AV1 4:4:4 is not offered and is rejected by dry-run validation."
        )
        addRow(
            to: root,
            label: "Total video bitrate",
            control: bitrateField,
            detail: "Bytes/second syntax accepted by Kyber (for example 100M); shared across active displays."
        )
        addRow(
            to: root,
            label: "Display mode",
            control: displayModePopup,
            detail: "Multiple displays map to independent Kymux streams."
        )
        addRow(
            to: root,
            labelView: displayValueLabel,
            control: displayValueField,
            detail: "Single index 0–15; multi-monitor count 2–8."
        )

        root.addArrangedSubview(sectionTitle("Audio and control"))
        addRow(
            to: root,
            label: "Audio",
            control: audioCheckbox,
            detail: "Requests Kyber's existing single Opus audio stream."
        )
        addRow(
            to: root,
            label: "Audio buffer",
            control: audioBufferField,
            detail: "Milliseconds (0–2000)."
        )
        addRow(
            to: root,
            label: "Kymux audio transport",
            control: audioTransportPopup,
            detail: "Uses the existing --kymux-audio modes."
        )
        addRow(
            to: root,
            label: "Input",
            control: inputsCheckbox,
            detail: "Controls mouse + focused-window keyboard forwarding via --inputs; off disables both."
        )
        addRow(
            to: root,
            label: "Immersive system-shortcut suppression",
            control: immersiveInputStatus,
            detail: "Kyber keyboard grab is forced off with --keyboard-grab=false."
        )

        let clipboardStatus = NSTextField(
            wrappingLabelWithString: "Unavailable in this macOS build"
        )
        clipboardStatus.textColor = .systemOrange
        clipboardStatus.font = .systemFont(ofSize: NSFont.systemFontSize, weight: .semibold)
        addRow(
            to: root,
            label: "Clipboard",
            control: clipboardStatus,
            detail: "No --clipboard argument is passed."
        )

        root.addArrangedSubview(validationLabel)
        constrainWidth(validationLabel, to: root)

        root.addArrangedSubview(sectionTitle("Exact child command"))
        root.addArrangedSubview(commandPreview)
        constrainWidth(commandPreview, to: root)
        commandPreview.heightAnchor.constraint(greaterThanOrEqualToConstant: 34).isActive = true

        let actionRow = NSStackView(views: [
            connectButton,
            disconnectButton,
            sessionStatus,
        ])
        actionRow.orientation = .horizontal
        actionRow.alignment = .centerY
        actionRow.spacing = 10
        root.addArrangedSubview(actionRow)

        let telemetryHeader = NSStackView()
        telemetryHeader.orientation = .horizontal
        telemetryHeader.alignment = .centerY
        telemetryHeader.spacing = 10
        let telemetryTitle = sectionTitle("Live child output + metrics.json + log/kyclient.log")
        let clearButton = NSButton(title: "Clear", target: self, action: #selector(clearTelemetry(_:)))
        clearButton.bezelStyle = .rounded
        telemetryHeader.addArrangedSubview(telemetryTitle)
        telemetryHeader.addArrangedSubview(clearButton)
        root.addArrangedSubview(telemetryHeader)

        let scrollView = NSScrollView()
        scrollView.hasVerticalScroller = true
        scrollView.hasHorizontalScroller = false
        scrollView.autohidesScrollers = true
        scrollView.borderType = .bezelBorder
        scrollView.documentView = telemetryView
        telemetryView.isVerticallyResizable = true
        telemetryView.isHorizontallyResizable = false
        telemetryView.autoresizingMask = [.width]
        telemetryView.textContainer?.widthTracksTextView = true
        root.addArrangedSubview(scrollView)
        constrainWidth(scrollView, to: root)
        scrollView.heightAnchor.constraint(greaterThanOrEqualToConstant: 190).isActive = true
    }

    private func sectionTitle(_ string: String) -> NSTextField {
        let label = NSTextField(labelWithString: string)
        label.font = .systemFont(ofSize: 13, weight: .semibold)
        return label
    }

    private func addRow(
        to root: NSStackView,
        label: String,
        control: NSView,
        detail: String
    ) {
        addRow(
            to: root,
            labelView: NSTextField(labelWithString: label),
            control: control,
            detailView: detailLabel(detail)
        )
    }

    private func addRow(
        to root: NSStackView,
        label: String,
        control: NSView,
        detailView: NSView
    ) {
        addRow(
            to: root,
            labelView: NSTextField(labelWithString: label),
            control: control,
            detailView: detailView
        )
    }

    private func addRow(
        to root: NSStackView,
        labelView: NSTextField,
        control: NSView,
        detail: String
    ) {
        addRow(to: root, labelView: labelView, control: control, detailView: detailLabel(detail))
    }

    private func addRow(
        to root: NSStackView,
        labelView: NSTextField,
        control: NSView,
        detailView: NSView
    ) {
        let row = NSStackView(views: [labelView, control, detailView])
        row.orientation = .horizontal
        row.alignment = .centerY
        row.spacing = 10

        labelView.alignment = .right
        labelView.widthAnchor.constraint(equalToConstant: 175).isActive = true
        control.widthAnchor.constraint(equalToConstant: 285).isActive = true
        detailView.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)

        root.addArrangedSubview(row)
        constrainWidth(row, to: root)
    }

    private func detailLabel(_ string: String) -> NSTextField {
        let label = NSTextField(wrappingLabelWithString: string)
        label.textColor = .secondaryLabelColor
        label.font = .systemFont(ofSize: NSFont.smallSystemFontSize)
        return label
    }

    private func constrainWidth(_ view: NSView, to root: NSView) {
        view.widthAnchor.constraint(equalTo: root.widthAnchor).isActive = true
    }

    private func loadPreferences() {
        let defaults = UserDefaults.standard
        defaults.register(defaults: [
            PreferenceKey.host: "",
            PreferenceKey.port: 8080,
            PreferenceKey.codec: CodecMode.h264420.rawValue,
            PreferenceKey.bitrate: "100M",
            PreferenceKey.displayMode: DisplayMode.single.rawValue,
            PreferenceKey.displayValue: 0,
            PreferenceKey.audioEnabled: false,
            PreferenceKey.audioBuffer: 40,
            PreferenceKey.audioTransport: KymuxAudioTransport.reliable.rawValue,
            PreferenceKey.inputsEnabled: true,
        ])

        hostField.stringValue = defaults.string(forKey: PreferenceKey.host) ?? ""
        portField.integerValue = defaults.integer(forKey: PreferenceKey.port)

        let codec = CodecMode(
            rawValue: defaults.string(forKey: PreferenceKey.codec) ?? ""
        ) ?? .h264420
        codecPopup.selectItem(at: CodecMode.allCases.firstIndex(of: codec) ?? 0)

        bitrateField.stringValue = defaults.string(forKey: PreferenceKey.bitrate) ?? "100M"

        let displayMode = DisplayMode(
            rawValue: defaults.string(forKey: PreferenceKey.displayMode) ?? ""
        ) ?? .single
        displayModePopup.selectItem(at: DisplayMode.allCases.firstIndex(of: displayMode) ?? 0)
        displayValueField.integerValue = defaults.integer(forKey: PreferenceKey.displayValue)

        audioCheckbox.state = defaults.bool(forKey: PreferenceKey.audioEnabled) ? .on : .off
        audioBufferField.integerValue = defaults.integer(forKey: PreferenceKey.audioBuffer)

        let audioTransport = KymuxAudioTransport(
            rawValue: defaults.string(forKey: PreferenceKey.audioTransport) ?? ""
        ) ?? .reliable
        audioTransportPopup.selectItem(
            at: KymuxAudioTransport.allCases.firstIndex(of: audioTransport) ?? 0
        )

        inputsCheckbox.state = defaults.bool(forKey: PreferenceKey.inputsEnabled) ? .on : .off
    }

    private func savePreferences(_ configuration: LauncherConfiguration) {
        let defaults = UserDefaults.standard
        defaults.set(configuration.host, forKey: PreferenceKey.host)
        defaults.set(configuration.port, forKey: PreferenceKey.port)
        defaults.set(configuration.codec.rawValue, forKey: PreferenceKey.codec)
        defaults.set(configuration.bitrate, forKey: PreferenceKey.bitrate)
        defaults.set(configuration.displayMode.rawValue, forKey: PreferenceKey.displayMode)
        defaults.set(configuration.displayValue, forKey: PreferenceKey.displayValue)
        defaults.set(configuration.audioEnabled, forKey: PreferenceKey.audioEnabled)
        defaults.set(configuration.audioBufferMilliseconds, forKey: PreferenceKey.audioBuffer)
        defaults.set(configuration.audioTransport.rawValue, forKey: PreferenceKey.audioTransport)
        defaults.set(configuration.inputsEnabled, forKey: PreferenceKey.inputsEnabled)
    }

    private func selectedCodec() -> CodecMode {
        let index = max(0, codecPopup.indexOfSelectedItem)
        return CodecMode.allCases[min(index, CodecMode.allCases.count - 1)]
    }

    private func selectedDisplayMode() -> DisplayMode {
        let index = max(0, displayModePopup.indexOfSelectedItem)
        return DisplayMode.allCases[min(index, DisplayMode.allCases.count - 1)]
    }

    private func selectedAudioTransport() -> KymuxAudioTransport {
        let index = max(0, audioTransportPopup.indexOfSelectedItem)
        return KymuxAudioTransport.allCases[
            min(index, KymuxAudioTransport.allCases.count - 1)
        ]
    }

    private func configurationFromControls() throws -> LauncherConfiguration {
        guard let port = Int(portField.stringValue) else {
            throw LauncherError(message: "Port must be an integer.")
        }
        guard let displayValue = Int(displayValueField.stringValue) else {
            throw LauncherError(message: "Display index/count must be an integer.")
        }
        guard let audioBuffer = Int(audioBufferField.stringValue) else {
            throw LauncherError(message: "Audio buffer must be an integer.")
        }

        return LauncherConfiguration(
            host: hostField.stringValue,
            port: port,
            codec: selectedCodec(),
            bitrate: bitrateField.stringValue,
            displayMode: selectedDisplayMode(),
            displayValue: displayValue,
            audioEnabled: audioCheckbox.state == .on,
            audioBufferMilliseconds: audioBuffer,
            audioTransport: selectedAudioTransport(),
            inputsEnabled: inputsCheckbox.state == .on
        )
    }

    private func updateDerivedControls() {
        let codec = selectedCodec()
        chromaValue.stringValue = codec.chroma
        codecQualification.stringValue = codec.qualification

        let displayMode = selectedDisplayMode()
        displayValueLabel.stringValue =
            displayMode == .single ? "Display index" : "Display count"
        if displayMode == .multiple && displayValueField.integerValue < 2 {
            displayValueField.integerValue = 2
        }

        let audioEnabled = audioCheckbox.state == .on
        audioBufferField.isEnabled = audioEnabled
        audioTransportPopup.isEnabled = audioEnabled
    }

    private func updateCommandPreview() {
        do {
            let arguments = try KyclientArguments.build(for: configurationFromControls())
            commandPreview.stringValue = commandDescription(
                executable: LauncherPaths.childExecutable,
                arguments: arguments
            )
            validationLabel.stringValue = ""
            connectButton.isEnabled = child == nil
        } catch {
            commandPreview.stringValue = "Command unavailable until the fields are valid."
            validationLabel.stringValue = error.localizedDescription
            connectButton.isEnabled = false
        }
    }

    @objc private func controlChanged(_ sender: Any?) {
        updateDerivedControls()
        updateCommandPreview()
        persistCurrentPreferences()
    }

    func controlTextDidChange(_ notification: Notification) {
        updateCommandPreview()
        persistCurrentPreferences()
    }

    @objc private func connectClicked(_ sender: Any?) {
        guard child == nil else {
            return
        }

        do {
            let configuration = try configurationFromControls().validated()
            let arguments = try KyclientArguments.build(for: configuration)
            let runtimeDirectory = try LauncherPaths.createApplicationSupportDirectory()
            let executable = LauncherPaths.childExecutable

            guard FileManager.default.isExecutableFile(atPath: executable.path) else {
                throw LauncherError(
                    message: "Streaming child is missing or not executable: \(executable.path)"
                )
            }

            savePreferences(configuration)
            try startChild(
                executable: executable,
                arguments: arguments,
                runtimeDirectory: runtimeDirectory
            )
        } catch {
            validationLabel.stringValue = error.localizedDescription
            sessionStatus.stringValue = "Launch failed"
            appendTranscript("[launcher error] \(error.localizedDescription)\n")
        }
    }

    private func startChild(
        executable: URL,
        arguments: [String],
        runtimeDirectory: URL
    ) throws {
        let process = Process()
        let stdout = Pipe()
        let stderr = Pipe()

        process.executableURL = executable
        process.arguments = arguments
        process.currentDirectoryURL = runtimeDirectory
        process.standardOutput = stdout
        process.standardError = stderr

        configurePipe(stdout, label: "stdout")
        configurePipe(stderr, label: "stderr")

        disconnectRequested = false
        tailCursors.removeAll()
        appendTranscript(
            "\n[launcher] cwd=\(runtimeDirectory.path)\n"
                + "[launcher] command=\(commandDescription(executable: executable, arguments: arguments))\n"
                + "[launcher] TLS identity verification is disabled for this internal prototype.\n"
        )

        process.terminationHandler = { [weak self, weak process] _ in
            guard let self, let process else {
                return
            }
            DispatchQueue.main.async {
                self.childDidTerminate(process)
            }
        }

        do {
            try process.run()
        } catch {
            stdout.fileHandleForReading.readabilityHandler = nil
            stderr.fileHandleForReading.readabilityHandler = nil
            throw error
        }

        child = process
        stdoutPipe = stdout
        stderrPipe = stderr
        connectButton.isEnabled = false
        disconnectButton.isEnabled = true
        sessionStatus.stringValue = "Running (PID \(process.processIdentifier))"
        validationLabel.stringValue = ""
        startTelemetryTimer()
    }

    private func configurePipe(_ pipe: Pipe, label: String) {
        pipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let data = handle.availableData
            if data.isEmpty {
                handle.readabilityHandler = nil
                return
            }
            let text = String(decoding: data, as: UTF8.self)
            DispatchQueue.main.async {
                self?.appendTranscript("[child \(label)] \(text)")
            }
        }
    }

    @objc private func disconnectClicked(_ sender: Any?) {
        guard let process = child, process.isRunning else {
            return
        }

        disconnectRequested = true
        sessionStatus.stringValue = "Disconnecting…"
        disconnectButton.isEnabled = false
        appendTranscript("[launcher] disconnect requested\n")
        process.terminate()

        DispatchQueue.main.asyncAfter(deadline: .now() + 3) { [weak self, weak process] in
            guard let self,
                  let process,
                  self.child === process,
                  process.isRunning
            else {
                return
            }
            self.appendTranscript("[launcher] child did not stop after 3 seconds; sending SIGKILL\n")
            Darwin.kill(process.processIdentifier, SIGKILL)
        }
    }

    private func childDidTerminate(_ process: Process) {
        guard child === process else {
            return
        }

        tailTelemetryFiles()
        telemetryTimer?.invalidate()
        telemetryTimer = nil
        stdoutPipe?.fileHandleForReading.readabilityHandler = nil
        stderrPipe?.fileHandleForReading.readabilityHandler = nil
        stdoutPipe = nil
        stderrPipe = nil
        child = nil

        let requested = disconnectRequested
        disconnectRequested = false
        connectButton.isEnabled = true
        disconnectButton.isEnabled = false
        sessionStatus.stringValue =
            requested
                ? "Disconnected"
                : "Exited with status \(process.terminationStatus)"
        appendTranscript(
            "[launcher] child exited: reason=\(process.terminationReason.rawValue) "
                + "status=\(process.terminationStatus)\n"
        )
        updateCommandPreview()
    }

    private func startTelemetryTimer() {
        telemetryTimer?.invalidate()
        telemetryTimer = Timer.scheduledTimer(
            timeInterval: 0.5,
            target: self,
            selector: #selector(telemetryTimerFired(_:)),
            userInfo: nil,
            repeats: true
        )
        tailTelemetryFiles()
    }

    @objc private func telemetryTimerFired(_ timer: Timer) {
        tailTelemetryFiles()
    }

    private func tailTelemetryFiles() {
        let root = LauncherPaths.applicationSupportDirectory
        tailFile(root.appendingPathComponent("metrics.json"), label: "metrics.json")
        tailFile(
            root
                .appendingPathComponent("log", isDirectory: true)
                .appendingPathComponent("kyclient.log"),
            label: "log/kyclient.log"
        )
    }

    private func tailFile(_ url: URL, label: String) {
        guard let attributes = try? FileManager.default.attributesOfItem(atPath: url.path),
              let sizeNumber = attributes[.size] as? NSNumber
        else {
            return
        }

        let size = sizeNumber.uint64Value
        var cursor = tailCursors[url] ?? TailCursor()
        if !cursor.initialized {
            cursor.initialized = true
            cursor.offset = size > initialTailBytes ? size - initialTailBytes : 0
        } else if size < cursor.offset {
            cursor.offset = 0
        }

        guard size > cursor.offset else {
            tailCursors[url] = cursor
            return
        }

        let count = min(size - cursor.offset, maximumTailReadBytes)
        do {
            let handle = try FileHandle(forReadingFrom: url)
            defer {
                try? handle.close()
            }
            try handle.seek(toOffset: cursor.offset)
            let data = try handle.read(upToCount: Int(count)) ?? Data()
            cursor.offset += UInt64(data.count)
            tailCursors[url] = cursor
            if !data.isEmpty {
                appendTranscript("[\(label)] \(String(decoding: data, as: UTF8.self))")
            }
        } catch {
            tailCursors[url] = cursor
        }
    }

    private func appendTranscript(_ text: String) {
        if telemetryView.string == "No session output yet.\n" {
            transcript = ""
        }
        transcript.append(text)
        if transcript.count > maximumTranscriptCharacters {
            transcript = "… earlier output truncated …\n"
                + String(transcript.suffix(maximumTranscriptCharacters))
        }
        telemetryView.string = transcript
        telemetryView.scrollToEndOfDocument(nil)
    }

    @objc private func clearTelemetry(_ sender: Any?) {
        transcript = ""
        telemetryView.string = ""
    }
}

@MainActor
private final class LauncherAppDelegate: NSObject, NSApplicationDelegate {
    private var controller: LauncherController?

    func applicationDidFinishLaunching(_ notification: Notification) {
        configureMainMenu()
        let controller = LauncherController()
        self.controller = controller
        controller.show()
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }

    func applicationWillTerminate(_ notification: Notification) {
        controller?.persistCurrentPreferences()
        controller?.terminateChildForApplicationExit()
    }

    private func configureMainMenu() {
        let mainMenu = NSMenu()
        let applicationItem = NSMenuItem()
        mainMenu.addItem(applicationItem)

        let applicationMenu = NSMenu()
        applicationMenu.addItem(
            withTitle: "About ReplayDesktop",
            action: #selector(NSApplication.orderFrontStandardAboutPanel(_:)),
            keyEquivalent: ""
        )
        applicationMenu.addItem(.separator())
        applicationMenu.addItem(
            withTitle: "Quit ReplayDesktop",
            action: #selector(NSApplication.terminate(_:)),
            keyEquivalent: "q"
        )
        applicationItem.submenu = applicationMenu
        NSApp.mainMenu = mainMenu
    }
}

private struct DryRunOptions {
    var host = "127.0.0.1"
    var port = 8080
    var codec = "h264"
    var chroma = "420"
    var bitrate = "100M"
    var displayMode = DisplayMode.single
    var displayValue = 0
    var audioEnabled = false
    var audioBufferMilliseconds = 40
    var audioTransport = KymuxAudioTransport.reliable
    var inputsEnabled = true

    init(arguments: [String]) throws {
        for argument in arguments where argument != "--dry-run" {
            guard argument.hasPrefix("--"),
                  let equals = argument.firstIndex(of: "=")
            else {
                throw LauncherError(
                    message: "Dry-run options must use --name=value syntax: \(argument)"
                )
            }

            let name = String(argument[argument.index(argument.startIndex, offsetBy: 2) ..< equals])
            let value = String(argument[argument.index(after: equals)...])
            switch name {
            case "host":
                host = value
            case "port":
                guard let parsed = Int(value) else {
                    throw LauncherError(message: "Invalid --port value '\(value)'.")
                }
                port = parsed
            case "codec":
                switch value.lowercased() {
                case "h264", "h.264":
                    codec = "h264"
                case "hevc", "h265", "h.265":
                    codec = "hevc"
                case "av1":
                    codec = "av1"
                case "h264420":
                    codec = "h264"
                    chroma = "420"
                case "hevc420":
                    codec = "hevc"
                    chroma = "420"
                case "hevc444":
                    codec = "hevc"
                    chroma = "444"
                case "av1420":
                    codec = "av1"
                    chroma = "420"
                case "av1444":
                    codec = "av1"
                    chroma = "444"
                default:
                    throw LauncherError(message: "Invalid --codec value '\(value)'.")
                }
            case "chroma":
                chroma = value
            case "bitrate":
                bitrate = value
            case "display-mode":
                guard let parsed = DisplayMode(rawValue: value) else {
                    throw LauncherError(message: "Invalid --display-mode value '\(value)'.")
                }
                displayMode = parsed
            case "display-index":
                guard let parsed = Int(value) else {
                    throw LauncherError(message: "Invalid --display-index value '\(value)'.")
                }
                displayMode = .single
                displayValue = parsed
            case "display-count":
                guard let parsed = Int(value) else {
                    throw LauncherError(message: "Invalid --display-count value '\(value)'.")
                }
                displayMode = .multiple
                displayValue = parsed
            case "audio":
                audioEnabled = try parseBoolean(value, option: name)
            case "audio-buffer":
                guard let parsed = Int(value) else {
                    throw LauncherError(message: "Invalid --audio-buffer value '\(value)'.")
                }
                audioBufferMilliseconds = parsed
            case "kymux-audio":
                guard let parsed = KymuxAudioTransport(rawValue: value) else {
                    throw LauncherError(message: "Invalid --kymux-audio value '\(value)'.")
                }
                audioTransport = parsed
            case "inputs":
                inputsEnabled = try parseBoolean(value, option: name)
            case "keyboard-grab":
                throw LauncherError(
                    message:
                        "Immersive system-shortcut suppression is unavailable on macOS; "
                        + "--keyboard-grab=false is always emitted."
                )
            case "clipboard":
                throw LauncherError(
                    message: "Clipboard is unavailable in this macOS build and cannot be enabled."
                )
            default:
                throw LauncherError(message: "Unknown dry-run option '--\(name)'.")
            }
        }
    }

    var configuration: LauncherConfiguration {
        get throws {
            LauncherConfiguration(
                host: host,
                port: port,
                codec: try CodecMode.parse(codec: codec, chroma: chroma),
                bitrate: bitrate,
                displayMode: displayMode,
                displayValue: displayValue,
                audioEnabled: audioEnabled,
                audioBufferMilliseconds: audioBufferMilliseconds,
                audioTransport: audioTransport,
                inputsEnabled: inputsEnabled
            )
        }
    }
}

private func parseBoolean(_ value: String, option: String) throws -> Bool {
    switch value.lowercased() {
    case "true", "1", "yes":
        return true
    case "false", "0", "no":
        return false
    default:
        throw LauncherError(message: "Invalid --\(option) boolean '\(value)'.")
    }
}

private func require(_ condition: @autoclosure () -> Bool, _ message: String) throws {
    guard condition() else {
        throw LauncherError(message: "Self-test assertion failed: \(message)")
    }
}

private func runSelfTest() -> Int32 {
    do {
        try require(
            KyclientArguments.fixedBaseline == [
                "--port=8080",
                "--protocol=kymux",
                "--tls-skip-verification",
                "--video-buffer=0",
                "--metrics=true",
                "--auto-reconnect=false",
            ],
            "fixed baseline changed"
        )

        var combinations = 0
        for codec in CodecMode.allCases {
            for displayMode in DisplayMode.allCases {
                for audioEnabled in [false, true] {
                    for transport in KymuxAudioTransport.allCases {
                        for inputsEnabled in [false, true] {
                            let configuration = LauncherConfiguration(
                                host: "test-host.invalid",
                                port: 8080,
                                codec: codec,
                                bitrate: "100M",
                                displayMode: displayMode,
                                displayValue: displayMode == .single ? 0 : 2,
                                audioEnabled: audioEnabled,
                                audioBufferMilliseconds: 40,
                                audioTransport: transport,
                                inputsEnabled: inputsEnabled
                            )
                            let arguments = try KyclientArguments.build(for: configuration)
                            combinations += 1

                            try require(
                                Array(arguments.prefix(KyclientArguments.fixedBaseline.count))
                                    == KyclientArguments.fixedBaseline,
                                "default command did not begin with the exact baseline"
                            )
                            try require(
                                arguments.contains("--video-codec=\(codec.codecArgument)"),
                                "codec mapping missing for \(codec.rawValue)"
                            )
                            try require(
                                arguments.contains("--444") == codec.usesYUV444,
                                "4:4:4 flag mismatch for \(codec.rawValue)"
                            )
                            try require(
                                arguments.contains(
                                    displayMode == .single
                                        ? "--display-idx=0"
                                        : "--display-count=2"
                                ),
                                "display mapping missing"
                            )
                            try require(
                                arguments.contains("--audio=\(audioEnabled)"),
                                "audio mapping missing"
                            )
                            try require(
                                arguments.contains("--kymux-audio=\(transport.rawValue)"),
                                "audio transport mapping missing"
                            )
                            try require(
                                arguments.contains("--inputs=\(inputsEnabled)"),
                                "input toggle did not control mouse + focused keyboard forwarding"
                            )
                            try require(
                                arguments.filter { $0.hasPrefix("--keyboard-grab=") }
                                    == [MacOSInputPolicy.keyboardGrabArgument],
                                "keyboard grab must be forced off exactly once"
                            )
                            try require(
                                !arguments.contains(where: { $0.hasPrefix("--clipboard") }),
                                "clipboard flag must never be emitted"
                            )
                            try require(
                                arguments.contains("--protocol=kymux"),
                                "multi-monitor and all GUI sessions must use Kymux"
                            )
                            try require(
                                Array(arguments.suffix(2)) == ["--", "test-host.invalid"],
                                "positional host was not protected by the option delimiter"
                            )
                        }
                    }
                }
            }
        }

        var rejectedAV1444 = false
        do {
            _ = try CodecMode.parse(codec: "av1", chroma: "444")
        } catch {
            rejectedAV1444 = true
        }
        try require(rejectedAV1444, "AV1 4:4:4 was not rejected")

        var customPort = LauncherConfiguration()
        customPort.host = "test-host.invalid"
        customPort.port = 9_999
        let customPortArguments = try KyclientArguments.build(for: customPort)
        try require(
            customPortArguments.first == "--port=9999"
                && !customPortArguments.contains("--port=8080"),
            "GUI port did not replace the baseline slot cleanly"
        )

        var rejectedOptionHost = false
        do {
            var optionHost = LauncherConfiguration()
            optionHost.host = "--clipboard=true"
            _ = try KyclientArguments.build(for: optionHost)
        } catch {
            rejectedOptionHost = true
        }
        try require(rejectedOptionHost, "option-shaped host was not rejected")

        try require(
            !MacOSInputPolicy.immersiveSystemShortcutSuppressionAvailable
                && MacOSInputPolicy.immersiveStatus == "Unavailable on macOS",
            "UI policy claimed immersive system-shortcut suppression"
        )

        var rejectedKeyboardGrabRequest = false
        do {
            _ = try DryRunOptions(
                arguments: ["--dry-run", "--keyboard-grab=true"]
            )
        } catch {
            rejectedKeyboardGrabRequest = true
        }
        try require(
            rejectedKeyboardGrabRequest,
            "launcher dry run accepted a working keyboard-grab request"
        )

        print(
            "SELF-TEST PASS: \(combinations) codec/display/audio/transport combinations; "
                + "input toggles mouse + focused keyboard; immersive grab unavailable; "
                + "AV1 4:4:4 and option-shaped hosts rejected; clipboard omitted"
        )
        return 0
    } catch {
        fputs("SELF-TEST FAIL: \(error.localizedDescription)\n", stderr)
        return 1
    }
}

private func runDryRun(arguments: [String]) -> Int32 {
    do {
        let options = try DryRunOptions(arguments: arguments)
        let configuration = try options.configuration.validated()
        let childArguments = try KyclientArguments.build(for: configuration)

        print("DRY-RUN OK")
        print("executable=\(LauncherPaths.childExecutable.path)")
        print("cwd=\(LauncherPaths.applicationSupportDirectory.path)")
        print("argv-count=\(childArguments.count)")
        for (index, argument) in childArguments.enumerated() {
            print("argv[\(index)]=\(argument)")
        }
        print(
            "command="
                + commandDescription(
                    executable: LauncherPaths.childExecutable,
                    arguments: childArguments
                )
        )
        return 0
    } catch {
        fputs("DRY-RUN ERROR: \(error.localizedDescription)\n", stderr)
        return 2
    }
}

private func printRuntimeDirectory() -> Int32 {
    do {
        let directory = try LauncherPaths.createApplicationSupportDirectory()
        print(directory.path)
        return 0
    } catch {
        fputs("RUNTIME-DIRECTORY ERROR: \(error.localizedDescription)\n", stderr)
        return 1
    }
}

private func printLauncherHelp() {
    print(
        """
        ReplayDesktopLauncher

        Run without arguments to open the native technical connection panel.

        Script modes:
          --self-test
          --print-runtime-directory
          --dry-run [--host=value] [--port=value]
                    [--codec=h264|hevc|av1|h264420|hevc420|hevc444|av1420|av1444]
                    [--chroma=420|444] [--bitrate=100M]
                    [--display-index=0 | --display-count=2]
                    [--audio=true|false] [--audio-buffer=40]
                    [--kymux-audio=reliable|unreliable|unreliable_fec]
                    [--inputs=true|false]

        Input controls mouse + focused-window keyboard forwarding.
        Immersive system-shortcut suppression is unavailable on macOS;
        --keyboard-grab=false is always emitted and grab requests are rejected.
        AV1 4:4:4 and clipboard requests are rejected.
        """
    )
}

@main
@MainActor
private enum ReplayDesktopLauncherMain {
    static func main() {
        let arguments = Array(CommandLine.arguments.dropFirst())

        if arguments.contains("--self-test") {
            exit(runSelfTest())
        }
        if arguments == ["--print-runtime-directory"] {
            exit(printRuntimeDirectory())
        }
        if arguments.contains("--dry-run") {
            exit(runDryRun(arguments: arguments))
        }
        if arguments.contains("--help") || arguments.contains("-h") {
            printLauncherHelp()
            exit(0)
        }
        if !arguments.isEmpty {
            fputs(
                "ReplayDesktopLauncher: unknown option(s): \(arguments.joined(separator: " "))\n",
                stderr
            )
            fputs("Use --help for launcher options; use Contents/MacOS/kyclient for raw CLI.\n", stderr)
            exit(2)
        }

        let application = NSApplication.shared
        application.setActivationPolicy(.regular)
        let delegate = LauncherAppDelegate()
        application.delegate = delegate
        application.run()
        _ = delegate
    }
}
