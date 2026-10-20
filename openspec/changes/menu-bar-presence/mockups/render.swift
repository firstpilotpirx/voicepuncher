// Renders the menu bar mockups of `design-spec.md` with the real SF Symbols and the system font:
//   swift render.swift <out-dir>
// Every picture is drawn at 2x. The menu bar is drawn as a solid approximation of its
// translucent look (light: #ECECEC, dark: #2C2C2C).
import AppKit

let scale: CGFloat = 2
let barHeight: CGFloat = 24
let barWidth: CGFloat = 220
let iconSide: CGFloat = 22
let symbolSize: CGFloat = 16

struct Appearance {
    let name: String
    let bar: NSColor
    let ink: NSColor
    let menu: NSColor
    let disabled: NSColor
    let separator: NSColor
}
let light = Appearance(name: "light", bar: NSColor(white: 0.925, alpha: 1), ink: .black,
                       menu: NSColor(white: 0.965, alpha: 1), disabled: NSColor(white: 0.55, alpha: 1),
                       separator: NSColor(white: 0.82, alpha: 1))
let dark = Appearance(name: "dark", bar: NSColor(white: 0.17, alpha: 1), ink: .white,
                      menu: NSColor(white: 0.19, alpha: 1), disabled: NSColor(white: 0.55, alpha: 1),
                      separator: NSColor(white: 0.32, alpha: 1))

enum Icon {
    case template(String)
    case variable(String, Double)
    case recordingBadge        // mic.fill + red dot
    case recordingCircle       // record.circle in system red
}

struct State {
    let file: String
    let title: String
    let symbol: String
    let icon: Icon
}

let states: [State] = [
    State(file: "01-ready", title: "Ready", symbol: "mic", icon: .template("mic")),
    State(file: "02-recording", title: "Recording", symbol: "mic.fill + red dot", icon: .recordingBadge),
    State(file: "03-processing", title: "Recognizing / Refining", symbol: "waveform (variable)", icon: .variable("waveform", 0.66)),
    State(file: "04-waiting-focus", title: "Waiting for a text field", symbol: "keyboard", icon: .template("keyboard")),
    State(file: "05-listening", title: "Voice control listening", symbol: "ear", icon: .template("ear")),
    State(file: "06-commands-unavailable", title: "Commands unavailable", symbol: "ear.trianglebadge.\u{200B}exclamationmark", icon: .template("ear.trianglebadge.exclamationmark")),
    State(file: "07-warning", title: "Failure notice", symbol: "exclamationmark.triangle", icon: .template("exclamationmark.triangle")),
    State(file: "08-waiting", title: "Starting / Restarting", symbol: "hourglass", icon: .template("hourglass")),
    State(file: "09-failed", title: "Failed", symbol: "exclamationmark.triangle.fill", icon: .template("exclamationmark.triangle.fill")),
]

func symbolImage(_ name: String, size: CGFloat, color: NSColor, variable: Double? = nil) -> NSImage {
    let base: NSImage
    if let v = variable {
        base = NSImage(systemSymbolName: name, variableValue: v, accessibilityDescription: name)!
    } else {
        base = NSImage(systemSymbolName: name, accessibilityDescription: name)!
    }
    let mono = base.withSymbolConfiguration(NSImage.SymbolConfiguration(pointSize: size, weight: .medium))!
    // Tint through the alpha mask, as AppKit draws a template image: a cut-out stays a cut-out.
    let tinted = NSImage(size: mono.size)
    tinted.lockFocus()
    mono.draw(in: NSRect(origin: .zero, size: mono.size))
    color.set()
    NSRect(origin: .zero, size: mono.size).fill(using: .sourceAtop)
    tinted.unlockFocus()
    return tinted
}

func drawCentered(_ image: NSImage, in rect: NSRect) {
    let s = image.size
    let origin = NSPoint(x: rect.midX - s.width / 2, y: rect.midY - s.height / 2)
    image.draw(in: NSRect(origin: origin, size: s))
}

func drawIcon(_ icon: Icon, in rect: NSRect, ink: NSColor) {
    switch icon {
    case .template(let name):
        drawCentered(symbolImage(name, size: symbolSize, color: ink), in: rect)
    case .variable(let name, let v):
        drawCentered(symbolImage(name, size: symbolSize, color: ink, variable: v), in: rect)
    case .recordingBadge:
        drawCentered(symbolImage("mic.fill", size: symbolSize, color: ink), in: rect)
        let d: CGFloat = 7
        let dot = NSBezierPath(ovalIn: NSRect(x: rect.maxX - d - 1, y: rect.minY + 2, width: d, height: d))
        NSColor.systemRed.setFill(); dot.fill()
    case .recordingCircle:
        drawCentered(symbolImage("record.circle", size: 17, color: .systemRed), in: rect)
    }
}

/// A picture `size` points large, drawn at 2x, saved as PNG.
func render(_ size: NSSize, to path: String, _ draw: (NSRect) -> Void) {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: Int(size.width * scale), pixelsHigh: Int(size.height * scale),
                               bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                               colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    rep.size = size
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    draw(NSRect(origin: .zero, size: size))
    NSGraphicsContext.restoreGraphicsState()
    try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: path))
}

/// The right part of the menu bar: Wi-Fi, the voicepuncher icon, battery, the clock.
func drawBar(_ rect: NSRect, _ a: Appearance, icon: Icon, highlighted: Bool = false) -> NSRect {
    a.bar.setFill(); rect.fill()
    var x = rect.minX + 12
    let row = NSRect(x: 0, y: rect.minY + (rect.height - iconSide) / 2, width: iconSide, height: iconSide)
    drawCentered(symbolImage("wifi", size: symbolSize, color: a.ink), in: NSRect(x: x, y: row.minY, width: iconSide, height: iconSide))
    x += iconSide + 14
    let iconRect = NSRect(x: x, y: row.minY, width: iconSide, height: iconSide)
    if highlighted {
        let hl = NSBezierPath(roundedRect: iconRect.insetBy(dx: -4, dy: -2), xRadius: 4, yRadius: 4)
        NSColor(white: a.name == "light" ? 0 : 1, alpha: 0.18).setFill(); hl.fill()
    }
    drawIcon(icon, in: iconRect, ink: a.ink)
    x += iconSide + 14
    drawCentered(symbolImage("battery.75percent", size: symbolSize, color: a.ink), in: NSRect(x: x, y: row.minY, width: 28, height: iconSide))
    x += 28 + 12
    let clock = NSAttributedString(string: "Mon 20 Oct  13:45", attributes: [
        .font: NSFont.systemFont(ofSize: 13, weight: .medium), .foregroundColor: a.ink])
    clock.draw(at: NSPoint(x: x, y: rect.minY + (rect.height - clock.size().height) / 2 + 1))
    return iconRect
}

struct MenuRow { let text: String; let shortcut: String?; let disabled: Bool }
let separatorRow = MenuRow(text: "-", shortcut: nil, disabled: false)

func menuRows(status: String) -> [MenuRow] {
    [MenuRow(text: "voicepuncher 0.27.0", shortcut: nil, disabled: true),
     MenuRow(text: status, shortcut: nil, disabled: true),
     separatorRow,
     MenuRow(text: "Settings…", shortcut: "⌘,", disabled: false),
     MenuRow(text: "Open Logs Folder", shortcut: nil, disabled: false),
     separatorRow,
     MenuRow(text: "Quit voicepuncher", shortcut: "⌘Q", disabled: false)]
}

let rowHeight: CGFloat = 22
let separatorHeight: CGFloat = 9

func drawMenu(_ rows: [MenuRow], below iconRect: NSRect, _ a: Appearance) {
    let height = rows.reduce(CGFloat(10)) { $0 + ($1.text == "-" ? separatorHeight : rowHeight) }
    let longest = rows.map { NSAttributedString(string: $0.text, attributes: [.font: NSFont.systemFont(ofSize: 13)]).size().width }.max()!
    let menuWidth = max(300, longest + 70)
    let origin = NSPoint(x: iconRect.minX - 8, y: iconRect.minY - 6 - height)
    let frame = NSRect(x: origin.x, y: origin.y, width: menuWidth, height: height)
    let panel = NSBezierPath(roundedRect: frame, xRadius: 8, yRadius: 8)
    NSColor(white: 0, alpha: 0.25).setFill()
    NSBezierPath(roundedRect: frame.offsetBy(dx: 0, dy: -2).insetBy(dx: -1, dy: -1), xRadius: 9, yRadius: 9).fill()
    a.menu.setFill(); panel.fill()
    a.separator.setStroke(); panel.lineWidth = 0.5; panel.stroke()
    var y = frame.maxY - 5
    for row in rows {
        if row.text == "-" {
            let yy = y - separatorHeight / 2
            a.separator.setFill()
            NSRect(x: frame.minX + 10, y: yy, width: frame.width - 20, height: 1).fill()
            y -= separatorHeight
            continue
        }
        let color = row.disabled ? a.disabled : a.ink
        let text = NSAttributedString(string: row.text, attributes: [.font: NSFont.systemFont(ofSize: 13), .foregroundColor: color])
        text.draw(at: NSPoint(x: frame.minX + 14, y: y - rowHeight + (rowHeight - text.size().height) / 2))
        if let sc = row.shortcut {
            let s = NSAttributedString(string: sc, attributes: [.font: NSFont.systemFont(ofSize: 13), .foregroundColor: a.disabled])
            s.draw(at: NSPoint(x: frame.maxX - 14 - s.size().width, y: y - rowHeight + (rowHeight - s.size().height) / 2))
        }
        y -= rowHeight
    }
}

let out = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "."
try? FileManager.default.createDirectory(atPath: out + "/states", withIntermediateDirectories: true)

// 1. One picture per icon state, light and dark.
for s in states {
    for a in [light, dark] {
        render(NSSize(width: barWidth, height: barHeight), to: "\(out)/states/\(s.file)--\(a.name).png") { r in
            _ = drawBar(r, a, icon: s.icon)
        }
    }
}

// 2. The three processing frames side by side; the two recording variants.
render(NSSize(width: 3 * (barWidth / 2) + 20, height: barHeight), to: "\(out)/states/03-processing-frames.png") { r in
    light.bar.setFill(); r.fill()
    for (i, v) in [0.33, 0.66, 1.0].enumerated() {
        let rect = NSRect(x: 20 + CGFloat(i) * (barWidth / 2), y: 1, width: iconSide, height: iconSide)
        drawIcon(.variable("waveform", v), in: rect, ink: light.ink)
    }
}
render(NSSize(width: barWidth * 2, height: barHeight * 2), to: "\(out)/states/02-recording-variants.png") { r in
    let half = barWidth
    for (col, icon) in [Icon.recordingBadge, .recordingCircle].enumerated() {
        for (rowI, a) in [light, dark].enumerated() {
            let rect = NSRect(x: CGFloat(col) * half, y: CGFloat(rowI) * barHeight, width: half, height: barHeight)
            _ = drawBar(rect, a, icon: icon)
        }
    }
}

// 3. The menu in three states.
let menus: [(String, String, Icon)] = [
    ("10-menu-ready", "Ready — hold Right Option to dictate", .template("mic")),
    ("11-menu-recording", "Recording — release Right Option to stop", .recordingBadge),
    ("12-menu-failed", "Failed: speech recognition service at http://127.0.0.1:8081 does not answer", .template("exclamationmark.triangle.fill")),
]
for (file, status, icon) in menus {
    for a in [light, dark] {
        let height: CGFloat = barHeight + 190
        render(NSSize(width: file == "12-menu-failed" ? 560 : barWidth + 160, height: height), to: "\(out)/states/\(file)--\(a.name).png") { r in
            (a.name == "light" ? NSColor(white: 0.85, alpha: 1) : NSColor(white: 0.1, alpha: 1)).setFill(); r.fill()
            let bar = NSRect(x: 0, y: r.maxY - barHeight, width: r.width, height: barHeight)
            let iconRect = drawBar(bar, a, icon: icon, highlighted: true)
            drawMenu(menuRows(status: status), below: iconRect, a)
        }
    }
}

// 4. The overview: every state with its name and symbol, light and dark.
let cell = NSSize(width: 330, height: 62)
render(NSSize(width: cell.width * 2 + 30, height: cell.height * CGFloat(states.count) + 50), to: "\(out)/menu-bar-overview.png") { r in
    NSColor.white.setFill(); r.fill()
    let heading = NSAttributedString(string: "voicepuncher menu bar icon states", attributes: [.font: NSFont.boldSystemFont(ofSize: 15), .foregroundColor: NSColor.black])
    heading.draw(at: NSPoint(x: 15, y: r.maxY - 30))
    for (i, s) in states.enumerated() {
        for (col, a) in [light, dark].enumerated() {
            let x = 15 + CGFloat(col) * cell.width
            let top = r.maxY - 45 - CGFloat(i) * cell.height
            let bar = NSRect(x: x, y: top - barHeight, width: barWidth, height: barHeight)
            _ = drawBar(bar, a, icon: s.icon)
            let label = NSAttributedString(string: "\(s.title) — \(s.symbol)", attributes: [.font: NSFont.systemFont(ofSize: 11), .foregroundColor: NSColor.darkGray])
            label.draw(at: NSPoint(x: x, y: top - barHeight - 16))
        }
    }
}
print("rendered into \(out)")
