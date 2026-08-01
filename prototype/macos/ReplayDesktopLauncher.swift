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

private struct BrokerLoginRequest: Encodable {
    let username: String
    let password: String
}

private struct BrokerRefreshRequest: Encodable {
    let refreshToken: String

    enum CodingKeys: String, CodingKey {
        case refreshToken = "refresh_token"
    }
}

private struct BrokerTokens: Decodable {
    let accessToken: String
    let accessExpiresAt: Int64
    let refreshToken: String
    let refreshExpiresAt: Int64

    enum CodingKeys: String, CodingKey {
        case accessToken = "access_token"
        case accessExpiresAt = "access_expires_at"
        case refreshToken = "refresh_token"
        case refreshExpiresAt = "refresh_expires_at"
    }
}

private struct BrokerWorkstation: Decodable {
    let id: UUID
    let name: String
    let hostname: String?
    let online: Bool
    let lastSeenAt: Int64?

    enum CodingKeys: String, CodingKey {
        case id, name, hostname, online
        case lastSeenAt = "last_seen_at"
    }
}

private struct BrokerLanSession: Decodable {
    let sessionID: UUID
    let status: String
    let directEndpoint: String
    let workstationCertificateSHA256: String
    let kyberToken: String
    let expiresAt: Int64

    enum CodingKeys: String, CodingKey {
        case status
        case sessionID = "session_id"
        case directEndpoint = "direct_endpoint"
        case workstationCertificateSHA256 = "workstation_certificate_sha256"
        case kyberToken = "kyber_token"
        case expiresAt = "expires_at"
    }
}

private struct BrokerAPIError: Decodable {
    let error: String
}

private enum BrokerAPI {
    static func normalizedBaseURL(_ input: String) throws -> URL {
        var value = input.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !value.isEmpty else {
            throw LauncherError(message: "Enter the broker hostname or IP address.")
        }
        if !value.contains("://") {
            value = "http://" + value
        }
        guard var components = URLComponents(string: value),
              let scheme = components.scheme?.lowercased(),
              ["http", "https"].contains(scheme),
              components.host != nil,
              components.user == nil,
              components.password == nil,
              components.query == nil,
              components.fragment == nil,
              components.path.isEmpty || components.path == "/"
        else {
            throw LauncherError(
                message: "Broker must be an HTTP(S) hostname or IP without credentials, a path, or a query."
            )
        }
        components.scheme = scheme
        components.path = ""
        if scheme == "http", components.port == nil {
            components.port = 8_090
        }
        guard let url = components.url else {
            throw LauncherError(message: "The broker URL is invalid.")
        }
        return url
    }

    static func login(
        baseURL: URL,
        username: String,
        password: String,
        completion: @escaping (Result<BrokerTokens, Error>) -> Void
    ) {
        request(
            baseURL: baseURL,
            path: "v1/auth/login",
            method: "POST",
            bearer: nil,
            body: BrokerLoginRequest(username: username, password: password),
            completion: completion
        )
    }

    static func refresh(
        baseURL: URL,
        refreshToken: String,
        completion: @escaping (Result<BrokerTokens, Error>) -> Void
    ) {
        request(
            baseURL: baseURL,
            path: "v1/auth/refresh",
            method: "POST",
            bearer: nil,
            body: BrokerRefreshRequest(refreshToken: refreshToken),
            completion: completion
        )
    }

    static func workstations(
        baseURL: URL,
        accessToken: String,
        completion: @escaping (Result<[BrokerWorkstation], Error>) -> Void
    ) {
        request(
            baseURL: baseURL,
            path: "v1/workstations",
            method: "GET",
            bearer: accessToken,
            body: Optional<BrokerLoginRequest>.none,
            completion: completion
        )
    }

    static func createLanSession(
        baseURL: URL,
        workstationID: UUID,
        accessToken: String,
        completion: @escaping (Result<BrokerLanSession, Error>) -> Void
    ) {
        request(
            baseURL: baseURL,
            path: "v1/workstations/\(workstationID.uuidString)/lan-sessions",
            method: "POST",
            bearer: accessToken,
            body: Optional<BrokerLoginRequest>.none,
            completion: completion
        )
    }

    static func logout(baseURL: URL, accessToken: String) {
        request(
            baseURL: baseURL,
            path: "v1/auth/logout",
            method: "POST",
            bearer: accessToken,
            body: Optional<BrokerLoginRequest>.none
        ) { (_: Result<EmptyBrokerResponse, Error>) in }
    }

    static func closeSession(baseURL: URL, sessionID: UUID, accessToken: String) {
        request(
            baseURL: baseURL,
            path: "v1/sessions/\(sessionID.uuidString)",
            method: "DELETE",
            bearer: accessToken,
            body: Optional<BrokerLoginRequest>.none
        ) { (_: Result<EmptyBrokerResponse, Error>) in }
    }

    private struct EmptyBrokerResponse: Decodable {}

    private static func request<Response: Decodable, Body: Encodable>(
        baseURL: URL,
        path: String,
        method: String,
        bearer: String?,
        body: Body?,
        completion: @escaping (Result<Response, Error>) -> Void
    ) {
        let url = baseURL.appendingPathComponent(path)
        var request = URLRequest(url: url)
        request.httpMethod = method
        request.timeoutInterval = 10
        request.setValue("application/json", forHTTPHeaderField: "accept")
        if let bearer {
            request.setValue("Bearer \(bearer)", forHTTPHeaderField: "authorization")
        }
        do {
            if let body {
                request.httpBody = try JSONEncoder().encode(body)
                request.setValue("application/json", forHTTPHeaderField: "content-type")
            }
        } catch {
            completion(.failure(error))
            return
        }
        URLSession.shared.dataTask(with: request) { data, response, error in
            if let error {
                completion(.failure(error))
                return
            }
            guard let response = response as? HTTPURLResponse else {
                completion(.failure(LauncherError(message: "Broker returned no HTTP response.")))
                return
            }
            let data = data ?? Data()
            guard (200 ... 299).contains(response.statusCode) else {
                let detail = (try? JSONDecoder().decode(BrokerAPIError.self, from: data).error)
                    ?? "HTTP \(response.statusCode)"
                completion(.failure(LauncherError(message: "Broker request failed: \(detail)")))
                return
            }
            if Response.self == EmptyBrokerResponse.self, data.isEmpty {
                completion(.success(EmptyBrokerResponse() as! Response))
                return
            }
            do {
                completion(.success(try JSONDecoder().decode(Response.self, from: data)))
            } catch {
                completion(.failure(error))
            }
        }.resume()
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
    var clipboardEnabled = false
    var kyberToken: String?
    var tlsFingerprint: String?

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
            guard result.displayValue == 2 else {
                throw LauncherError(message: "This prototype supports exactly 2 displays.")
            }
        }

        guard (0 ... 2_000).contains(result.audioBufferMilliseconds) else {
            throw LauncherError(message: "Audio buffer must be between 0 and 2000 ms.")
        }

        if let token = result.kyberToken {
            let segments = token.split(separator: ".", omittingEmptySubsequences: false)
            if token.count > 4_096 || segments.count != 3 || segments.contains(where: \.isEmpty) {
                throw LauncherError(message: "The broker returned an invalid Kyber token.")
            }
        }
        if let fingerprint = result.tlsFingerprint,
           !isValidKyclientFingerprint(fingerprint)
        {
            throw LauncherError(message: "The broker returned an invalid TLS fingerprint.")
        }

        return result
    }
}

private func isValidKyclientFingerprint(_ value: String) -> Bool {
    let components = value.split(separator: ":", omittingEmptySubsequences: false)
    let hexadecimal = CharacterSet(charactersIn: "0123456789ABCDEF")
    return components.count == 33
        && components.first == "sha256"
        && components.dropFirst().allSatisfy { component in
            component.count == 2
                && component.unicodeScalars.allSatisfy { hexadecimal.contains($0) }
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
        if let fingerprint = configuration.tlsFingerprint {
            arguments.removeAll { $0 == "--tls-skip-verification" }
            arguments.append("--tls-fingerprint=\(fingerprint)")
        }
        if let token = configuration.kyberToken {
            arguments.append("--auth-token=\(token)")
        }

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
        arguments.append("--clipboard=\(configuration.clipboardEnabled)")
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
    let redacted = arguments.map { argument in
        argument.hasPrefix("--auth-token=") ? "--auth-token=<redacted>" : argument
    }
    return ([executable.path] + redacted).map(shellQuoted).joined(separator: " ")
}

private enum PreferenceKey {
    static let brokerURL = "broker.url"
    static let brokerUsername = "broker.username"
    static let lastWorkstationID = "broker.lastWorkstationID"
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
    static let clipboardEnabled = "prototype.clipboardEnabled"
}

private struct TailCursor {
    var initialized = false
    var offset: UInt64 = 0
}

private enum TerminalFrameOutcome: String, Equatable {
    case displayed = "Displayed"
    case skipped = "Skipped"
}

private struct CurrentMetrics: Equatable {
    var hostAcquiredToEncodedMicros: Int64?
    var hostEncodedToSentMicros: Int64?
    var networkLocalRTTMicros: Int64?
    var networkRemoteRTTMicros: Int64?
    var networkPingDelayMicros: Int64?
    var networkLocalPacketsLost: Int64?
    var networkRemotePacketsLost: Int64?
    var networkLocalDroppedPackets: Int64?
    var networkRemoteDroppedPackets: Int64?
    var clientReceivedToDecodedMicros: Int64?
    var clientDecodedToDisplayedMicros: Int64?
    var terminalFrameOutcome: TerminalFrameOutcome?

    var isUnknown: Bool {
        hostAcquiredToEncodedMicros == nil
            && hostEncodedToSentMicros == nil
            && networkLocalRTTMicros == nil
            && networkRemoteRTTMicros == nil
            && networkPingDelayMicros == nil
            && networkLocalPacketsLost == nil
            && networkRemotePacketsLost == nil
            && networkLocalDroppedPackets == nil
            && networkRemoteDroppedPackets == nil
            && clientReceivedToDecodedMicros == nil
            && clientDecodedToDisplayedMicros == nil
            && terminalFrameOutcome == nil
    }
}

private struct MetricsJSONLReducer {
    private enum VideoEvent: String {
        case acquired
        case encoding
        case encoded
        case sent
        case received
        case decoding
        case decoded
        case prepared
        case displayed
        case skipped
    }

    private struct FrameKey: Hashable {
        let sourceID: UInt32
        let pts: UInt64
    }

    private struct FrameMetrics {
        var timestamps: [VideoEvent: Int64] = [:]
    }

    private(set) var current = CurrentMetrics()
    private var frames: [FrameKey: FrameMetrics] = [:]
    private var frameOrder: [FrameKey] = []
    private var incompleteLine = Data()

    private let maximumRecentFrames = 256

    mutating func reset() {
        current = CurrentMetrics()
        frames.removeAll(keepingCapacity: true)
        frameOrder.removeAll(keepingCapacity: true)
        incompleteLine.removeAll(keepingCapacity: true)
    }

    mutating func consume(_ chunk: Data) {
        guard !chunk.isEmpty else {
            return
        }

        incompleteLine.append(chunk)
        var lineStart = incompleteLine.startIndex
        while let newline = incompleteLine[lineStart...].firstIndex(of: 0x0A) {
            consumeLine(Data(incompleteLine[lineStart ..< newline]))
            lineStart = incompleteLine.index(after: newline)
        }
        if lineStart != incompleteLine.startIndex {
            incompleteLine.removeSubrange(incompleteLine.startIndex ..< lineStart)
        }
    }

    private mutating func consumeLine(_ line: Data) {
        guard !line.isEmpty,
              let value = try? JSONSerialization.jsonObject(with: line),
              let object = value as? [String: Any],
              object["log_type"] as? String == "raw_metric",
              let type = object["type"] as? String
        else {
            return
        }

        switch type {
        case "video":
            consumeVideo(object)
        case "network_local", "network_remote":
            consumeNetworkStats(object, type: type)
        case "network_ping":
            consumeNetworkPing(object)
        default:
            return
        }
    }

    private mutating func consumeVideo(_ object: [String: Any]) {
        guard let eventName = object["event"] as? String,
              let event = VideoEvent(rawValue: eventName),
              let sourceIDValue = unsignedInteger(object["source_id"]),
              sourceIDValue <= UInt64(UInt32.max),
              let pts = unsignedInteger(object["pts"]),
              let timestamp = signedInteger(object["ts"])
        else {
            return
        }

        let key = FrameKey(sourceID: UInt32(sourceIDValue), pts: pts)
        var frame = frames[key] ?? FrameMetrics()
        if frames[key] == nil {
            frameOrder.append(key)
        }
        frame.timestamps[event] = timestamp
        frames[key] = frame

        if let duration = nonnegativeDelta(
            from: frame.timestamps[.acquired],
            to: frame.timestamps[.encoded]
        ) {
            current.hostAcquiredToEncodedMicros = duration
        }
        if let duration = nonnegativeDelta(
            from: frame.timestamps[.encoded],
            to: frame.timestamps[.sent]
        ) {
            current.hostEncodedToSentMicros = duration
        }
        if let duration = nonnegativeDelta(
            from: frame.timestamps[.received],
            to: frame.timestamps[.decoded]
        ) {
            current.clientReceivedToDecodedMicros = duration
        }
        if let duration = nonnegativeDelta(
            from: frame.timestamps[.decoded],
            to: frame.timestamps[.displayed]
        ) {
            current.clientDecodedToDisplayedMicros = duration
        }

        switch event {
        case .displayed:
            current.terminalFrameOutcome = .displayed
        case .skipped:
            current.terminalFrameOutcome = .skipped
        case .acquired, .encoding, .encoded, .sent, .received, .decoding, .decoded, .prepared:
            break
        }

        while frameOrder.count > maximumRecentFrames {
            frames.removeValue(forKey: frameOrder.removeFirst())
        }
    }

    private mutating func consumeNetworkStats(_ object: [String: Any], type: String) {
        guard let rtt = signedInteger(object["rtt_micros"]),
              let packetsLost = signedInteger(object["packets_lost"]),
              let droppedPackets = signedInteger(object["dropped_packets"]),
              rtt >= -1,
              packetsLost >= -1,
              droppedPackets >= -1
        else {
            return
        }

        let measuredRTT = rtt == -1 ? nil : rtt
        let measuredPacketsLost = packetsLost == -1 ? nil : packetsLost
        let measuredDroppedPackets = droppedPackets == -1 ? nil : droppedPackets

        if type == "network_local" {
            current.networkLocalRTTMicros = measuredRTT
            current.networkLocalPacketsLost = measuredPacketsLost
            current.networkLocalDroppedPackets = measuredDroppedPackets
        } else {
            current.networkRemoteRTTMicros = measuredRTT
            current.networkRemotePacketsLost = measuredPacketsLost
            current.networkRemoteDroppedPackets = measuredDroppedPackets
        }
    }

    private mutating func consumeNetworkPing(_ object: [String: Any]) {
        guard signedInteger(object["offset_micros"]) != nil,
              let delay = signedInteger(object["delay_micros"]),
              delay >= -1
        else {
            return
        }
        current.networkPingDelayMicros = delay == -1 ? nil : delay
    }

    private func signedInteger(_ value: Any?) -> Int64? {
        guard let value, !(value is Bool), let number = value as? NSNumber else {
            return nil
        }
        return Int64(number.stringValue)
    }

    private func unsignedInteger(_ value: Any?) -> UInt64? {
        guard let value, !(value is Bool), let number = value as? NSNumber else {
            return nil
        }
        return UInt64(number.stringValue)
    }

    private func nonnegativeDelta(from start: Int64?, to end: Int64?) -> Int64? {
        guard let start, let end else {
            return nil
        }
        let (duration, overflow) = end.subtractingReportingOverflow(start)
        guard !overflow, duration >= 0 else {
            return nil
        }
        return duration
    }
}

private enum MetricsCardField: Hashable {
    case hostAcquiredToEncoded
    case hostEncodedToSent
    case networkLocalRTT
    case networkRemoteRTT
    case networkPingDelay
    case networkLocalPacketsLost
    case networkRemotePacketsLost
    case networkLocalDroppedPackets
    case networkRemoteDroppedPackets
    case clientReceivedToDecoded
    case clientDecodedToDisplayed
    case terminalFrameOutcome

    var title: String {
        switch self {
        case .hostAcquiredToEncoded:
            return "Acquired → encoded"
        case .hostEncodedToSent:
            return "Encoded → sent"
        case .networkLocalRTT:
            return "Local RTT"
        case .networkRemoteRTT:
            return "Remote RTT"
        case .networkPingDelay:
            return "Ping delay"
        case .networkLocalPacketsLost:
            return "Local packets lost"
        case .networkRemotePacketsLost:
            return "Remote packets lost"
        case .networkLocalDroppedPackets:
            return "Local dropped"
        case .networkRemoteDroppedPackets:
            return "Remote dropped"
        case .clientReceivedToDecoded:
            return "Received → decoded"
        case .clientDecodedToDisplayed:
            return "Decoded → displayed"
        case .terminalFrameOutcome:
            return "Latest frame"
        }
    }
}

@MainActor
private final class LauncherController: NSObject, NSTextFieldDelegate, NSWindowDelegate {
    private let window: NSWindow
    private let brokerURLField = NSTextField()
    private let brokerUsernameField = NSTextField()
    private let brokerPasswordField = NSSecureTextField()
    private let brokerStatus = NSTextField(labelWithString: "Enter a broker and log in")
    private let loginButton = NSButton(title: "Log In", target: nil, action: nil)
    private let logoutButton = NSButton(title: "Log Out", target: nil, action: nil)
    private let workstationPopup = NSPopUpButton(frame: .zero, pullsDown: false)
    private let refreshWorkstationsButton = NSButton(title: "Refresh", target: nil, action: nil)
    private let settingsButton = NSButton(title: "⚙︎ Settings", target: nil, action: nil)
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
    private let clipboardCheckbox = NSButton(
        checkboxWithTitle: "Clipboard sync — Text + HTML, 60 KiB",
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
    private var metricsValueLabels: [MetricsCardField: NSTextField] = [:]

    private var child: Process?
    private var stdoutPipe: Pipe?
    private var stderrPipe: Pipe?
    private var telemetryTimer: Timer?
    private var tailCursors: [URL: TailCursor] = [:]
    private var metricsReducer = MetricsJSONLReducer()
    private var transcript = ""
    private var disconnectRequested = false
    private var brokerBaseURL: URL?
    private var brokerTokens: BrokerTokens?
    private var workstations: [BrokerWorkstation] = []
    private var workstationRefreshTimer: Timer?
    private var brokerRequestInFlight = false
    private var currentKyberToken: String?
    private var currentTLSFingerprint: String?
    private var currentBrokerSessionID: UUID?

    private let maximumTranscriptCharacters = 120_000
    private let initialTailBytes: UInt64 = 32 * 1_024
    private let maximumTailReadBytes: UInt64 = 64 * 1_024

    override init() {
        window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 1_000, height: 920),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )
        super.init()

        configureControls()
        buildInterface()
        renderCurrentMetrics()
        loadPreferences()
        updateDerivedControls()
        updateBrokerControls()
        updateCommandPreview()

        window.title = "ReplayDesktop"
        window.minSize = NSSize(width: 880, height: 720)
        window.isReleasedWhenClosed = false
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
        workstationRefreshTimer?.invalidate()
        workstationRefreshTimer = nil
        telemetryTimer?.invalidate()
        telemetryTimer = nil
        closeCurrentBrokerSession()
        if let child, child.isRunning {
            child.terminate()
        }
    }

    private func configureControls() {
        brokerURLField.placeholderString = "192.168.33.42:8090"
        brokerURLField.delegate = self
        brokerURLField.setAccessibilityLabel("Broker hostname or IP address")
        brokerUsernameField.placeholderString = "finn"
        brokerUsernameField.delegate = self
        brokerUsernameField.setAccessibilityLabel("Broker username")
        brokerPasswordField.placeholderString = "Password"
        brokerPasswordField.delegate = self
        brokerPasswordField.setAccessibilityLabel("Broker password")

        brokerStatus.textColor = .secondaryLabelColor
        loginButton.target = self
        loginButton.action = #selector(loginClicked(_:))
        loginButton.bezelStyle = .rounded
        logoutButton.target = self
        logoutButton.action = #selector(logoutClicked(_:))
        logoutButton.bezelStyle = .rounded
        logoutButton.isEnabled = false
        workstationPopup.target = self
        workstationPopup.action = #selector(workstationChanged(_:))
        workstationPopup.isEnabled = false
        refreshWorkstationsButton.target = self
        refreshWorkstationsButton.action = #selector(refreshWorkstationsClicked(_:))
        refreshWorkstationsButton.bezelStyle = .rounded
        refreshWorkstationsButton.isEnabled = false
        settingsButton.target = self
        settingsButton.action = #selector(showSettingsAndStatistics(_:))
        settingsButton.bezelStyle = .rounded

        hostField.placeholderString = "Direct LAN/VPN hostname or IP"
        hostField.delegate = self
        hostField.setAccessibilityLabel("Linux host name or IP address")
        hostField.isEditable = false

        portField.delegate = self
        portField.alignment = .right
        portField.setAccessibilityLabel("Kyber port")
        portField.isEditable = false

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
        clipboardCheckbox.target = self
        clipboardCheckbox.action = #selector(controlChanged(_:))
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

        let pageScrollView = NSScrollView()
        pageScrollView.translatesAutoresizingMaskIntoConstraints = false
        pageScrollView.hasVerticalScroller = true
        pageScrollView.hasHorizontalScroller = false
        pageScrollView.autohidesScrollers = true
        pageScrollView.drawsBackground = false
        content.addSubview(pageScrollView)

        let page = NSView()
        page.translatesAutoresizingMaskIntoConstraints = false
        pageScrollView.documentView = page

        let root = NSStackView()
        root.orientation = .vertical
        root.alignment = .leading
        root.spacing = 10
        root.translatesAutoresizingMaskIntoConstraints = false
        page.addSubview(root)

        NSLayoutConstraint.activate([
            pageScrollView.leadingAnchor.constraint(equalTo: content.leadingAnchor),
            pageScrollView.trailingAnchor.constraint(equalTo: content.trailingAnchor),
            pageScrollView.topAnchor.constraint(equalTo: content.topAnchor),
            pageScrollView.bottomAnchor.constraint(equalTo: content.bottomAnchor),
            page.widthAnchor.constraint(equalTo: pageScrollView.contentView.widthAnchor),
            page.heightAnchor.constraint(greaterThanOrEqualTo: pageScrollView.contentView.heightAnchor),
            root.leadingAnchor.constraint(equalTo: page.leadingAnchor, constant: 20),
            root.trailingAnchor.constraint(equalTo: page.trailingAnchor, constant: -20),
            root.topAnchor.constraint(equalTo: page.topAnchor, constant: 18),
            root.bottomAnchor.constraint(equalTo: page.bottomAnchor, constant: -18),
        ])

        let title = NSTextField(labelWithString: "ReplayDesktop")
        title.font = .systemFont(ofSize: 22, weight: .semibold)
        root.addArrangedSubview(title)

        let subtitle = NSTextField(
            wrappingLabelWithString:
                "Log in to your broker, choose an online workstation, then connect over the direct "
                + "Kymux LAN path. The broker never carries the desktop stream."
        )
        subtitle.textColor = .secondaryLabelColor
        root.addArrangedSubview(subtitle)
        constrainWidth(subtitle, to: root)

        let trustWarning = NSTextField(
            wrappingLabelWithString:
                "LAN development profile: broker HTTP is unencrypted. The direct Kyber connection "
                + "uses the workstation fingerprint and a short-lived broker-signed JWT."
        )
        trustWarning.textColor = .systemOrange
        trustWarning.font = .systemFont(ofSize: NSFont.systemFontSize, weight: .medium)
        root.addArrangedSubview(trustWarning)
        constrainWidth(trustWarning, to: root)

        root.addArrangedSubview(sectionTitle("Broker login"))
        addRow(
            to: root,
            label: "Broker hostname / IP",
            control: brokerURLField,
            detail: "Remembered after first login and always changeable here."
        )
        let credentials = NSStackView(views: [brokerUsernameField, brokerPasswordField])
        credentials.orientation = .horizontal
        credentials.distribution = .fillEqually
        credentials.spacing = 6
        addRow(
            to: root,
            label: "Account",
            control: credentials,
            detail: "Local test profile: finn / 1337. Password is never saved."
        )
        let loginActions = NSStackView(views: [loginButton, logoutButton, brokerStatus])
        loginActions.orientation = .horizontal
        loginActions.alignment = .centerY
        loginActions.spacing = 8
        addRow(
            to: root,
            label: "Broker session",
            control: loginActions,
            detail: "Access and refresh tokens remain in memory only."
        )

        root.addArrangedSubview(sectionTitle("Workstations"))
        let workstationActions = NSStackView(views: [workstationPopup, refreshWorkstationsButton])
        workstationActions.orientation = .horizontal
        workstationActions.alignment = .centerY
        workstationActions.spacing = 6
        addRow(
            to: root,
            label: "Available machines",
            control: workstationActions,
            detail: "● online, ○ offline. Stale hosts cannot start sessions."
        )

        root.addArrangedSubview(sectionTitle("Connection and video"))
        addRow(
            to: root,
            label: "Linux host / IP",
            control: hostField,
            detail: "Populated from the broker registration; media connects here directly."
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
            detail: "Single index 0–15; multi-monitor mode is fixed at 2 displays."
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

        addRow(
            to: root,
            label: "Clipboard",
            control: clipboardCheckbox,
            detail:
                "Experimental and opt-in. Host copy/paste policy is negotiated separately; "
                + "denied directions never access the Mac pasteboard."
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
            settingsButton,
            sessionStatus,
        ])
        actionRow.orientation = .horizontal
        actionRow.alignment = .centerY
        actionRow.spacing = 10
        root.addArrangedSubview(actionRow)

        let metricsCards = NSStackView(views: [
            makeMetricsCard(
                title: "Host",
                fields: [
                    .hostAcquiredToEncoded,
                    .hostEncodedToSent,
                ]
            ),
            makeMetricsCard(
                title: "Network",
                fields: [
                    .networkLocalRTT,
                    .networkRemoteRTT,
                    .networkPingDelay,
                    .networkLocalPacketsLost,
                    .networkRemotePacketsLost,
                    .networkLocalDroppedPackets,
                    .networkRemoteDroppedPackets,
                ]
            ),
            makeMetricsCard(
                title: "Client",
                fields: [
                    .clientReceivedToDecoded,
                    .clientDecodedToDisplayed,
                    .terminalFrameOutcome,
                ]
            ),
        ])
        metricsCards.orientation = .horizontal
        metricsCards.alignment = .top
        metricsCards.distribution = .fillEqually
        metricsCards.spacing = 10
        root.addArrangedSubview(metricsCards)
        constrainWidth(metricsCards, to: root)
        metricsCards.heightAnchor.constraint(equalToConstant: 132).isActive = true

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
        scrollView.heightAnchor.constraint(greaterThanOrEqualToConstant: 150).isActive = true
    }

    private func makeMetricsCard(
        title: String,
        fields: [MetricsCardField]
    ) -> NSView {
        let box = NSBox(frame: .zero)
        box.title = title
        box.titlePosition = .atTop
        box.boxType = .primary

        let container = NSView()
        box.contentView = container

        let values = NSStackView()
        values.orientation = .vertical
        values.alignment = .leading
        values.spacing = 2
        values.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(values)

        NSLayoutConstraint.activate([
            values.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 8),
            values.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -8),
            values.topAnchor.constraint(equalTo: container.topAnchor, constant: 4),
            values.bottomAnchor.constraint(lessThanOrEqualTo: container.bottomAnchor, constant: -4),
        ])

        for field in fields {
            let name = NSTextField(labelWithString: field.title)
            name.font = .systemFont(ofSize: NSFont.smallSystemFontSize)
            name.textColor = .secondaryLabelColor
            name.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)

            let value = NSTextField(labelWithString: "Unknown")
            value.font = .monospacedSystemFont(
                ofSize: NSFont.smallSystemFontSize,
                weight: .medium
            )
            value.alignment = .right
            value.setAccessibilityLabel("\(title) \(field.title)")
            value.setContentHuggingPriority(.required, for: .horizontal)
            value.setContentCompressionResistancePriority(.required, for: .horizontal)
            metricsValueLabels[field] = value

            let row = NSStackView(views: [name, value])
            row.orientation = .horizontal
            row.alignment = .centerY
            row.distribution = .fill
            row.spacing = 6
            values.addArrangedSubview(row)
            row.widthAnchor.constraint(equalTo: values.widthAnchor).isActive = true
        }

        return box
    }

    private func renderCurrentMetrics() {
        let metrics = metricsReducer.current
        setMetricValue(
            .hostAcquiredToEncoded,
            formattedMilliseconds(metrics.hostAcquiredToEncodedMicros)
        )
        setMetricValue(
            .hostEncodedToSent,
            formattedMilliseconds(metrics.hostEncodedToSentMicros)
        )
        setMetricValue(
            .networkLocalRTT,
            formattedMilliseconds(metrics.networkLocalRTTMicros)
        )
        setMetricValue(
            .networkRemoteRTT,
            formattedMilliseconds(metrics.networkRemoteRTTMicros)
        )
        setMetricValue(
            .networkPingDelay,
            formattedMilliseconds(metrics.networkPingDelayMicros)
        )
        setMetricValue(
            .networkLocalPacketsLost,
            formattedCounter(metrics.networkLocalPacketsLost)
        )
        setMetricValue(
            .networkRemotePacketsLost,
            formattedCounter(metrics.networkRemotePacketsLost)
        )
        setMetricValue(
            .networkLocalDroppedPackets,
            formattedCounter(metrics.networkLocalDroppedPackets)
        )
        setMetricValue(
            .networkRemoteDroppedPackets,
            formattedCounter(metrics.networkRemoteDroppedPackets)
        )
        setMetricValue(
            .clientReceivedToDecoded,
            formattedMilliseconds(metrics.clientReceivedToDecodedMicros)
        )
        setMetricValue(
            .clientDecodedToDisplayed,
            formattedMilliseconds(metrics.clientDecodedToDisplayedMicros)
        )
        setMetricValue(
            .terminalFrameOutcome,
            metrics.terminalFrameOutcome?.rawValue ?? "Unknown"
        )
    }

    private func setMetricValue(_ field: MetricsCardField, _ value: String) {
        metricsValueLabels[field]?.stringValue = value
    }

    private func formattedMilliseconds(_ microseconds: Int64?) -> String {
        guard let microseconds, microseconds >= 0 else {
            return "Unknown"
        }
        let wholeMilliseconds = microseconds / 1_000
        let remainder = microseconds % 1_000
        guard remainder != 0 else {
            return "\(wholeMilliseconds) ms"
        }

        var fraction = String(format: "%03lld", remainder)
        while fraction.last == "0" {
            fraction.removeLast()
        }
        return "\(wholeMilliseconds).\(fraction) ms"
    }

    private func formattedCounter(_ value: Int64?) -> String {
        guard let value, value >= 0 else {
            return "Unknown"
        }
        return String(value)
    }

    private func resetMetricsDashboard() {
        metricsReducer.reset()
        renderCurrentMetrics()
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
            PreferenceKey.brokerURL: "",
            PreferenceKey.brokerUsername: "finn",
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
            PreferenceKey.clipboardEnabled: false,
        ])

        brokerURLField.stringValue = defaults.string(forKey: PreferenceKey.brokerURL) ?? ""
        brokerUsernameField.stringValue =
            defaults.string(forKey: PreferenceKey.brokerUsername) ?? "finn"
        brokerPasswordField.stringValue = "1337"
        loadConnectionPreferences()
    }

    private func loadConnectionPreferences() {
        hostField.stringValue = connectionString(PreferenceKey.host) ?? ""
        portField.stringValue = String(connectionInteger(PreferenceKey.port))

        let codec = CodecMode(
            rawValue: connectionString(PreferenceKey.codec) ?? ""
        ) ?? .h264420
        codecPopup.selectItem(at: CodecMode.allCases.firstIndex(of: codec) ?? 0)

        bitrateField.stringValue = connectionString(PreferenceKey.bitrate) ?? "100M"

        let displayMode = DisplayMode(
            rawValue: connectionString(PreferenceKey.displayMode) ?? ""
        ) ?? .single
        displayModePopup.selectItem(at: DisplayMode.allCases.firstIndex(of: displayMode) ?? 0)
        displayValueField.stringValue = String(connectionInteger(PreferenceKey.displayValue))

        audioCheckbox.state = connectionBool(PreferenceKey.audioEnabled) ? .on : .off
        audioBufferField.stringValue = String(connectionInteger(PreferenceKey.audioBuffer))

        let audioTransport = KymuxAudioTransport(
            rawValue: connectionString(PreferenceKey.audioTransport) ?? ""
        ) ?? .reliable
        audioTransportPopup.selectItem(
            at: KymuxAudioTransport.allCases.firstIndex(of: audioTransport) ?? 0
        )

        inputsCheckbox.state = connectionBool(PreferenceKey.inputsEnabled) ? .on : .off
        clipboardCheckbox.state =
            connectionBool(PreferenceKey.clipboardEnabled) ? .on : .off
        updateDerivedControls()
    }

    private func savePreferences(_ configuration: LauncherConfiguration) {
        let defaults = UserDefaults.standard
        defaults.set(configuration.host, forKey: connectionPreferenceKey(PreferenceKey.host))
        defaults.set(configuration.port, forKey: connectionPreferenceKey(PreferenceKey.port))
        defaults.set(
            configuration.codec.rawValue,
            forKey: connectionPreferenceKey(PreferenceKey.codec)
        )
        defaults.set(configuration.bitrate, forKey: connectionPreferenceKey(PreferenceKey.bitrate))
        defaults.set(
            configuration.displayMode.rawValue,
            forKey: connectionPreferenceKey(PreferenceKey.displayMode)
        )
        defaults.set(
            configuration.displayValue,
            forKey: connectionPreferenceKey(PreferenceKey.displayValue)
        )
        defaults.set(
            configuration.audioEnabled,
            forKey: connectionPreferenceKey(PreferenceKey.audioEnabled)
        )
        defaults.set(
            configuration.audioBufferMilliseconds,
            forKey: connectionPreferenceKey(PreferenceKey.audioBuffer)
        )
        defaults.set(
            configuration.audioTransport.rawValue,
            forKey: connectionPreferenceKey(PreferenceKey.audioTransport)
        )
        defaults.set(
            configuration.inputsEnabled,
            forKey: connectionPreferenceKey(PreferenceKey.inputsEnabled)
        )
        defaults.set(
            configuration.clipboardEnabled,
            forKey: connectionPreferenceKey(PreferenceKey.clipboardEnabled)
        )
    }

    private func connectionPreferenceKey(_ base: String) -> String {
        guard let workstationID = selectedWorkstation()?.id else {
            return base
        }
        return "workstation.\(workstationID.uuidString).\(base)"
    }

    private func connectionObject(_ base: String) -> Any? {
        let defaults = UserDefaults.standard
        let scoped = connectionPreferenceKey(base)
        if scoped != base, let value = defaults.object(forKey: scoped) {
            return value
        }
        return defaults.object(forKey: base)
    }

    private func connectionString(_ base: String) -> String? {
        connectionObject(base) as? String
    }

    private func connectionInteger(_ base: String) -> Int {
        (connectionObject(base) as? NSNumber)?.intValue ?? 0
    }

    private func connectionBool(_ base: String) -> Bool {
        (connectionObject(base) as? NSNumber)?.boolValue ?? false
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
            inputsEnabled: inputsCheckbox.state == .on,
            clipboardEnabled: clipboardCheckbox.state == .on,
            kyberToken: currentKyberToken,
            tlsFingerprint: currentTLSFingerprint
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
            displayValueField.stringValue = "2"
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
                && !brokerRequestInFlight
                && brokerTokens != nil
                && selectedWorkstation()?.online == true
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

    private func selectedWorkstation() -> BrokerWorkstation? {
        let index = workstationPopup.indexOfSelectedItem
        guard index >= 0, index < workstations.count else {
            return nil
        }
        return workstations[index]
    }

    @objc private func loginClicked(_ sender: Any?) {
        guard !brokerRequestInFlight, child == nil else {
            return
        }
        do {
            let baseURL = try BrokerAPI.normalizedBaseURL(brokerURLField.stringValue)
            let username = brokerUsernameField.stringValue.trimmingCharacters(
                in: .whitespacesAndNewlines
            )
            let password = brokerPasswordField.stringValue
            guard !username.isEmpty, !password.isEmpty else {
                throw LauncherError(message: "Enter the broker username and password.")
            }
            brokerRequestInFlight = true
            brokerStatus.stringValue = "Logging in…"
            validationLabel.stringValue = ""
            updateBrokerControls()
            BrokerAPI.login(
                baseURL: baseURL,
                username: username,
                password: password
            ) { [weak self] result in
                DispatchQueue.main.async {
                    guard let self else { return }
                    self.brokerRequestInFlight = false
                    switch result {
                    case let .success(tokens):
                        self.brokerBaseURL = baseURL
                        self.brokerTokens = tokens
                        self.brokerURLField.stringValue = baseURL.absoluteString
                            .trimmingCharacters(in: CharacterSet(charactersIn: "/"))
                        UserDefaults.standard.set(
                            self.brokerURLField.stringValue,
                            forKey: PreferenceKey.brokerURL
                        )
                        UserDefaults.standard.set(
                            username,
                            forKey: PreferenceKey.brokerUsername
                        )
                        self.brokerStatus.stringValue = "Logged in as \(username)"
                        self.startWorkstationRefreshTimer()
                        self.loadWorkstations()
                    case let .failure(error):
                        self.brokerStatus.stringValue = "Login failed"
                        self.validationLabel.stringValue = error.localizedDescription
                        self.updateBrokerControls()
                    }
                }
            }
        } catch {
            validationLabel.stringValue = error.localizedDescription
            brokerStatus.stringValue = "Login unavailable"
        }
    }

    @objc private func logoutClicked(_ sender: Any?) {
        guard child == nil else {
            validationLabel.stringValue = "Disconnect before logging out."
            return
        }
        if let baseURL = brokerBaseURL, let accessToken = brokerTokens?.accessToken {
            BrokerAPI.logout(baseURL: baseURL, accessToken: accessToken)
        }
        workstationRefreshTimer?.invalidate()
        workstationRefreshTimer = nil
        brokerTokens = nil
        workstations = []
        workstationPopup.removeAllItems()
        currentKyberToken = nil
        currentTLSFingerprint = nil
        currentBrokerSessionID = nil
        brokerPasswordField.stringValue = ""
        brokerStatus.stringValue = "Logged out"
        updateBrokerControls()
        updateCommandPreview()
    }

    @objc private func refreshWorkstationsClicked(_ sender: Any?) {
        loadWorkstations()
    }

    private func startWorkstationRefreshTimer() {
        workstationRefreshTimer?.invalidate()
        workstationRefreshTimer = Timer.scheduledTimer(
            timeInterval: 15,
            target: self,
            selector: #selector(workstationRefreshTimerFired(_:)),
            userInfo: nil,
            repeats: true
        )
    }

    @objc private func workstationRefreshTimerFired(_ timer: Timer) {
        loadWorkstations()
    }

    private func loadWorkstations() {
        guard !brokerRequestInFlight,
              child == nil,
              let baseURL = brokerBaseURL,
              let tokens = brokerTokens
        else {
            return
        }
        let now = Int64(Date().timeIntervalSince1970)
        if tokens.accessExpiresAt <= now + 15 {
            guard tokens.refreshExpiresAt > now else {
                brokerStatus.stringValue = "Session expired — log in again"
                brokerTokens = nil
                updateBrokerControls()
                return
            }
            brokerRequestInFlight = true
            brokerStatus.stringValue = "Refreshing login…"
            updateBrokerControls()
            BrokerAPI.refresh(baseURL: baseURL, refreshToken: tokens.refreshToken) { [weak self] result in
                DispatchQueue.main.async {
                    guard let self else { return }
                    self.brokerRequestInFlight = false
                    switch result {
                    case let .success(refreshed):
                        self.brokerTokens = refreshed
                        self.loadWorkstations()
                    case let .failure(error):
                        self.brokerTokens = nil
                        self.brokerStatus.stringValue = "Session expired — log in again"
                        self.validationLabel.stringValue = error.localizedDescription
                        self.updateBrokerControls()
                    }
                }
            }
            return
        }
        brokerRequestInFlight = true
        brokerStatus.stringValue = "Refreshing machines…"
        updateBrokerControls()
        BrokerAPI.workstations(baseURL: baseURL, accessToken: tokens.accessToken) { [weak self] result in
            DispatchQueue.main.async {
                guard let self else { return }
                self.brokerRequestInFlight = false
                switch result {
                case let .success(workstations):
                    self.applyWorkstations(workstations)
                    let online = workstations.filter(\.online).count
                    self.brokerStatus.stringValue = "\(online) online / \(workstations.count) authorized"
                    self.validationLabel.stringValue = ""
                case let .failure(error):
                    self.brokerStatus.stringValue = "Machine refresh failed"
                    self.validationLabel.stringValue = error.localizedDescription
                }
                self.updateBrokerControls()
                self.updateCommandPreview()
            }
        }
    }

    private func applyWorkstations(_ updated: [BrokerWorkstation]) {
        let remembered = UserDefaults.standard.string(forKey: PreferenceKey.lastWorkstationID)
        let previous = selectedWorkstation()?.id.uuidString ?? remembered
        workstations = updated
        workstationPopup.removeAllItems()
        workstationPopup.addItems(withTitles: updated.map { workstation in
            "\(workstation.online ? "●" : "○")  \(workstation.name)"
        })
        let selection = updated.firstIndex { $0.id.uuidString == previous }
            ?? updated.firstIndex(where: \.online)
            ?? (updated.isEmpty ? nil : 0)
        if let selection {
            workstationPopup.selectItem(at: selection)
            workstationChanged(nil)
        }
    }

    @objc private func workstationChanged(_ sender: Any?) {
        guard let workstation = selectedWorkstation() else {
            updateCommandPreview()
            return
        }
        UserDefaults.standard.set(
            workstation.id.uuidString,
            forKey: PreferenceKey.lastWorkstationID
        )
        currentKyberToken = nil
        currentTLSFingerprint = nil
        currentBrokerSessionID = nil
        loadConnectionPreferences()
        hostField.stringValue = workstation.hostname ?? workstation.name
        if Int(portField.stringValue) == nil || portField.stringValue == "0" {
            portField.stringValue = "8080"
        }
        sessionStatus.stringValue = workstation.online ? "Ready to connect" : "Workstation offline"
        updateCommandPreview()
    }

    private func updateBrokerControls() {
        let loggedIn = brokerTokens != nil
        loginButton.isEnabled = !brokerRequestInFlight && !loggedIn && child == nil
        logoutButton.isEnabled = !brokerRequestInFlight && loggedIn && child == nil
        brokerURLField.isEnabled = !loggedIn && child == nil
        brokerUsernameField.isEnabled = !loggedIn && child == nil
        brokerPasswordField.isEnabled = !loggedIn && child == nil
        workstationPopup.isEnabled = loggedIn && !workstations.isEmpty && child == nil
        refreshWorkstationsButton.isEnabled = loggedIn && !brokerRequestInFlight && child == nil
    }

    @objc func showSettingsAndStatistics(_ sender: Any?) {
        show()
        if child != nil {
            validationLabel.stringValue =
                "Settings remain editable; transport changes apply to the next connection."
        }
    }

    func disconnectFromMenu() {
        disconnectClicked(nil)
    }

    @objc private func connectClicked(_ sender: Any?) {
        guard child == nil,
              !brokerRequestInFlight,
              let workstation = selectedWorkstation(),
              workstation.online,
              let baseURL = brokerBaseURL,
              let tokens = brokerTokens
        else {
            return
        }
        brokerRequestInFlight = true
        sessionStatus.stringValue = "Authorizing with broker…"
        updateBrokerControls()
        updateCommandPreview()
        BrokerAPI.createLanSession(
            baseURL: baseURL,
            workstationID: workstation.id,
            accessToken: tokens.accessToken
        ) { [weak self] result in
            DispatchQueue.main.async {
                guard let self else { return }
                self.brokerRequestInFlight = false
                switch result {
                case let .success(session):
                    do {
                        guard session.status == "ready",
                              session.expiresAt > Int64(Date().timeIntervalSince1970)
                        else {
                            throw LauncherError(
                                message: "Broker returned a session that is not ready or already expired."
                            )
                        }
                        let endpoint = try self.parseDirectEndpoint(session.directEndpoint)
                        self.hostField.stringValue = endpoint.host
                        self.portField.stringValue = String(endpoint.port)
                        self.currentKyberToken = session.kyberToken
                        self.currentTLSFingerprint = try self.kyclientFingerprint(
                            session.workstationCertificateSHA256
                        )
                        self.currentBrokerSessionID = session.sessionID
                        try self.launchSelectedSession()
                    } catch {
                        self.closeCurrentBrokerSession()
                        self.validationLabel.stringValue = error.localizedDescription
                        self.sessionStatus.stringValue = "Launch failed"
                    }
                case let .failure(error):
                    self.validationLabel.stringValue = error.localizedDescription
                    self.sessionStatus.stringValue = "Authorization failed"
                }
                self.updateBrokerControls()
                self.updateCommandPreview()
            }
        }
    }

    private func launchSelectedSession() throws {
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
    }

    private func parseDirectEndpoint(_ value: String) throws -> (host: String, port: Int) {
        guard let separator = value.lastIndex(of: ":"),
              separator != value.startIndex,
              let port = Int(value[value.index(after: separator)...]),
              (1 ... 65_535).contains(port)
        else {
            throw LauncherError(message: "Broker returned an invalid direct endpoint.")
        }
        return (String(value[..<separator]), port)
    }

    private func kyclientFingerprint(_ raw: String) throws -> String {
        let value = raw.lowercased()
        guard value.count == 64, value.allSatisfy({ $0.isHexDigit }) else {
            throw LauncherError(message: "Broker returned an invalid workstation fingerprint.")
        }
        var pairs: [String] = []
        var index = value.startIndex
        while index < value.endIndex {
            let next = value.index(index, offsetBy: 2)
            pairs.append(String(value[index ..< next]).uppercased())
            index = next
        }
        return "sha256:" + pairs.joined(separator: ":")
    }

    private func closeCurrentBrokerSession() {
        if let baseURL = brokerBaseURL,
           let accessToken = brokerTokens?.accessToken,
           let sessionID = currentBrokerSessionID
        {
            BrokerAPI.closeSession(
                baseURL: baseURL,
                sessionID: sessionID,
                accessToken: accessToken
            )
        }
        currentBrokerSessionID = nil
        currentKyberToken = nil
        currentTLSFingerprint = nil
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
        resetMetricsDashboard()
        appendTranscript(
            "\n[launcher] cwd=\(runtimeDirectory.path)\n"
                + "[launcher] command=\(commandDescription(executable: executable, arguments: arguments))\n"
                + "[launcher] TLS identity is pinned to the broker-enrolled workstation fingerprint.\n"
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
        updateBrokerControls()
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
        closeCurrentBrokerSession()
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
        resetMetricsDashboard()
        closeCurrentBrokerSession()

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
        updateBrokerControls()
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
                if label == "metrics.json" {
                    metricsReducer.consume(data)
                    renderCurrentMetrics()
                }
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
    private var statusItem: NSStatusItem?

    func applicationDidFinishLaunching(_ notification: Notification) {
        configureMainMenu()
        configureStatusItem()
        let controller = LauncherController()
        self.controller = controller
        controller.show()
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        false
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

        let sessionItem = NSMenuItem()
        mainMenu.addItem(sessionItem)
        let sessionMenu = NSMenu(title: "Session")
        let settingsItem = sessionMenu.addItem(
            withTitle: "Settings & Statistics",
            action: #selector(showSettingsAndStatistics(_:)),
            keyEquivalent: ","
        )
        settingsItem.target = self
        let disconnectItem = sessionMenu.addItem(
            withTitle: "Disconnect",
            action: #selector(disconnectSession(_:)),
            keyEquivalent: "d"
        )
        disconnectItem.keyEquivalentModifierMask = [.command, .shift]
        disconnectItem.target = self
        sessionItem.submenu = sessionMenu
        NSApp.mainMenu = mainMenu
    }

    private func configureStatusItem() {
        let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        statusItem.button?.title = "ReplayDesktop"
        statusItem.button?.toolTip = "ReplayDesktop session controls"

        let menu = NSMenu(title: "ReplayDesktop")
        let settingsItem = menu.addItem(
            withTitle: "Settings & Statistics",
            action: #selector(showSettingsAndStatistics(_:)),
            keyEquivalent: ""
        )
        settingsItem.target = self
        let disconnectItem = menu.addItem(
            withTitle: "Disconnect",
            action: #selector(disconnectSession(_:)),
            keyEquivalent: ""
        )
        disconnectItem.target = self
        menu.addItem(.separator())
        let quitItem = menu.addItem(
            withTitle: "Quit ReplayDesktop",
            action: #selector(NSApplication.terminate(_:)),
            keyEquivalent: ""
        )
        quitItem.target = NSApp
        statusItem.menu = menu
        self.statusItem = statusItem
    }

    @objc private func showSettingsAndStatistics(_ sender: Any?) {
        controller?.showSettingsAndStatistics(sender)
    }

    @objc private func disconnectSession(_ sender: Any?) {
        controller?.disconnectFromMenu()
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
    var clipboardEnabled = false

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
                clipboardEnabled = try parseBoolean(value, option: name)
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
                inputsEnabled: inputsEnabled,
                clipboardEnabled: clipboardEnabled
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

private func runMetricsReducerSelfTest() throws {
    func line(_ json: String) -> Data {
        Data((json + "\n").utf8)
    }

    var reducer = MetricsJSONLReducer()
    try require(reducer.current.isUnknown, "metrics did not initialize to Unknown")

    let encoded = line(
        #"{"log_type":"raw_metric","type":"video","event":"encoded","source_id":7,"pts":41,"ts":3000}"#
    )
    let split = encoded.count / 2
    reducer.consume(Data(encoded.prefix(split)))
    try require(reducer.current.isUnknown, "partial JSONL line changed metrics")
    reducer.consume(Data(encoded.dropFirst(split)))
    try require(
        reducer.current.hostAcquiredToEncodedMicros == nil,
        "encoded event alone fabricated a host duration"
    )

    reducer.consume(
        line(
            #"{"log_type":"raw_metric","type":"video","event":"acquired","source_id":7,"pts":41,"ts":1000}"#
        )
    )
    reducer.consume(
        line(
            #"{"log_type":"raw_metric","type":"video","event":"sent","source_id":7,"pts":41,"ts":4500}"#
        )
    )
    try require(
        reducer.current.hostAcquiredToEncodedMicros == 2_000
            && reducer.current.hostEncodedToSentMicros == 1_500,
        "out-of-order host events did not produce valid same-frame deltas"
    )

    reducer.consume(
        line(
            #"{"log_type":"raw_metric","type":"video","event":"displayed","source_id":7,"pts":41,"ts":11000}"#
        )
    )
    reducer.consume(
        line(
            #"{"log_type":"raw_metric","type":"video","event":"received","source_id":7,"pts":41,"ts":6000}"#
        )
    )
    reducer.consume(
        line(
            #"{"log_type":"raw_metric","type":"video","event":"decoded","source_id":7,"pts":41,"ts":9000}"#
        )
    )
    try require(
        reducer.current.clientReceivedToDecodedMicros == 3_000
            && reducer.current.clientDecodedToDisplayedMicros == 2_000
            && reducer.current.terminalFrameOutcome == .displayed,
        "out-of-order client events did not produce valid deltas and Displayed outcome"
    )

    reducer.consume(
        line(
            #"{"log_type":"raw_metric","type":"network_local","rtt_micros":1800,"packets_lost":7,"dropped_packets":3}"#
        )
    )
    try require(
        reducer.current.networkLocalRTTMicros == 1_800
            && reducer.current.networkLocalPacketsLost == 7
            && reducer.current.networkLocalDroppedPackets == 3,
        "valid local network counters were not accepted"
    )
    reducer.consume(
        line(
            #"{"log_type":"raw_metric","type":"network_local","rtt_micros":-1,"packets_lost":-1,"dropped_packets":-1}"#
        )
    )
    reducer.consume(
        line(
            #"{"log_type":"raw_metric","type":"network_remote","rtt_micros":-1,"packets_lost":-1,"dropped_packets":-1}"#
        )
    )
    try require(
        reducer.current.networkLocalRTTMicros == nil
            && reducer.current.networkLocalPacketsLost == nil
            && reducer.current.networkLocalDroppedPackets == nil
            && reducer.current.networkRemoteRTTMicros == nil
            && reducer.current.networkRemotePacketsLost == nil
            && reducer.current.networkRemoteDroppedPackets == nil,
        "network -1 sentinels did not remain Unknown"
    )

    reducer.consume(
        line(
            #"{"log_type":"raw_metric","type":"network_ping","offset_micros":-250,"delay_micros":2300}"#
        )
    )
    try require(
        reducer.current.networkPingDelayMicros == 2_300,
        "valid ping delay with a signed offset was not accepted"
    )

    let beforeMalformed = reducer.current
    reducer.consume(line(#"{"log_type":"raw_metric","type":"video","event":"encoded""#))
    reducer.consume(
        line(
            #"{"log_type":"frame_metric","type":"video","event":"encoded","source_id":7,"pts":41,"ts":9999}"#
        )
    )
    try require(
        reducer.current == beforeMalformed,
        "malformed or unsupported JSON changed current metrics"
    )

    reducer.consume(
        line(
            #"{"log_type":"raw_metric","type":"video","event":"skipped","source_id":7,"pts":42,"ts":12000}"#
        )
    )
    try require(
        reducer.current.terminalFrameOutcome == .skipped,
        "Skipped terminal outcome was not reported"
    )

    var negativeDeltaReducer = MetricsJSONLReducer()
    negativeDeltaReducer.consume(
        line(
            #"{"log_type":"raw_metric","type":"video","event":"acquired","source_id":1,"pts":1,"ts":5000}"#
        )
    )
    negativeDeltaReducer.consume(
        line(
            #"{"log_type":"raw_metric","type":"video","event":"encoded","source_id":1,"pts":1,"ts":4000}"#
        )
    )
    try require(
        negativeDeltaReducer.current.hostAcquiredToEncodedMicros == nil,
        "negative video delta became a measured value"
    )

    reducer.consume(Data(#"{"log_type":"raw_metric""#.utf8))
    reducer.reset()
    reducer.consume(Data(#","type":"network_ping","offset_micros":0,"delay_micros":1}"#.utf8))
    try require(reducer.current.isUnknown, "reset retained metrics or an incomplete JSONL line")
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
                            for clipboardEnabled in [false, true] {
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
                                    inputsEnabled: inputsEnabled,
                                    clipboardEnabled: clipboardEnabled
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
                                    arguments.filter { $0.hasPrefix("--clipboard=") }
                                        == ["--clipboard=\(clipboardEnabled)"],
                                    "clipboard toggle must emit exactly one independent flag"
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
        try require(
            customPortArguments.filter { $0.hasPrefix("--clipboard=") }
                == ["--clipboard=false"],
            "clipboard must be opt-in and default off"
        )

        let clipboardWithoutInput = try KyclientArguments.build(
            for: LauncherConfiguration(
                host: "test-host.invalid",
                inputsEnabled: false,
                clipboardEnabled: true
            )
        )
        try require(
            clipboardWithoutInput.contains("--inputs=false")
                && clipboardWithoutInput.contains("--clipboard=true"),
            "clipboard and general input toggles were conflated"
        )

        let brokerURL = try BrokerAPI.normalizedBaseURL("broker.lan")
        try require(
            brokerURL.absoluteString == "http://broker.lan:8090",
            "bare broker hostname did not select the LAN Docker port"
        )
        let secureBrokerURL = try BrokerAPI.normalizedBaseURL("https://broker.example")
        try require(
            secureBrokerURL.absoluteString == "https://broker.example",
            "explicit HTTPS broker URL changed unexpectedly"
        )
        var rejectedBrokerPath = false
        do {
            _ = try BrokerAPI.normalizedBaseURL("http://broker.lan:8090/not-allowed")
        } catch {
            rejectedBrokerPath = true
        }
        try require(rejectedBrokerPath, "broker URL accepted an unsupported path prefix")

        let brokerToken = "header.payload.signature"
        let fingerprint = "sha256:"
            + Array(repeating: "AA", count: 32).joined(separator: ":")
        let brokerArguments = try KyclientArguments.build(
            for: LauncherConfiguration(
                host: "broker-selected-host.invalid",
                kyberToken: brokerToken,
                tlsFingerprint: fingerprint
            )
        )
        try require(
            brokerArguments.contains("--auth-token=\(brokerToken)")
                && brokerArguments.contains("--tls-fingerprint=\(fingerprint)")
                && !brokerArguments.contains("--tls-skip-verification"),
            "broker session did not replace insecure TLS with JWT + fingerprint authentication"
        )
        let redactedCommand = commandDescription(
            executable: LauncherPaths.childExecutable,
            arguments: brokerArguments
        )
        try require(
            redactedCommand.contains("--auth-token=<redacted>")
                && !redactedCommand.contains(brokerToken),
            "broker JWT leaked into the command preview"
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

        var rejectedThirdDisplay = false
        do {
            _ = try KyclientArguments.build(
                for: LauncherConfiguration(
                    host: "test-host.invalid",
                    displayMode: .multiple,
                    displayValue: 3
                )
            )
        } catch {
            rejectedThirdDisplay = true
        }
        try require(
            rejectedThirdDisplay,
            "prototype accepted more than two simultaneous displays"
        )

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

        try runMetricsReducerSelfTest()

        print(
            "SELF-TEST PASS: \(combinations) codec/display/audio/transport combinations; "
                + "input toggles mouse + focused keyboard; immersive grab unavailable; "
                + "AV1 4:4:4, third displays, and option-shaped hosts rejected; "
                + "clipboard opt-in and independent; "
                + "broker URL, JWT redaction, and pinned TLS arguments passed; "
                + "metrics JSONL fixtures passed (Unknown/reset, split/out-of-order, "
                + "same-clock deltas, sentinels, malformed, skipped, negative rejection)"
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
                    [--inputs=true|false] [--clipboard=true|false]

        Input controls mouse + focused-window keyboard forwarding.
        Immersive system-shortcut suppression is unavailable on macOS;
        --keyboard-grab=false is always emitted and grab requests are rejected.
        Clipboard sync is experimental, limited to Text + HTML (60 KiB
        combined), negotiated directionally with the host, and defaults off.
        AV1 4:4:4 is rejected.
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
