// Vector rendering of Compose's two-page mark for the development app bundle.
import AppKit
let destination = CommandLine.arguments[1]
let folder = URL(fileURLWithPath: NSTemporaryDirectory()).appendingPathComponent(UUID().uuidString + ".iconset")
try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
defer { try? FileManager.default.removeItem(at: folder) }
func render(_ pixels: Int, _ filename: String) throws {
    let image = NSImage(size: NSSize(width: pixels, height: pixels))
    image.lockFocus()
    guard let cg = NSGraphicsContext.current?.cgContext else { return }
    cg.scaleBy(x: CGFloat(pixels)/1024, y: CGFloat(pixels)/1024)
    cg.setFillColor(NSColor(calibratedRed: 0.973, green: 0.974, blue: 0.959, alpha: 1).cgColor)
    cg.addPath(CGPath(roundedRect: CGRect(x: 40, y: 40, width: 944, height: 944), cornerWidth: 210, cornerHeight: 210, transform: nil))
    cg.fillPath()
    cg.setFillColor(NSColor(calibratedRed: 0.10, green: 0.12, blue: 0.10, alpha: 1).cgColor)
    for points in [[CGPoint(x:270,y:635), CGPoint(x:502,y:540), CGPoint(x:502,y:220), CGPoint(x:270,y:315)], [CGPoint(x:540.6,y:770.8),CGPoint(x:735.4,y:691),CGPoint(x:735.4,y:422.2),CGPoint(x:540.6,y:502)]] {
        cg.beginPath(); cg.move(to: points[0]); for p in points.dropFirst() { cg.addLine(to:p) }; cg.closePath(); cg.fillPath()
    }
    image.unlockFocus()
    let bitmap = NSBitmapImageRep(data: image.tiffRepresentation!)!
    try bitmap.representation(using: .png, properties: [:])!.write(to: folder.appendingPathComponent(filename))
}
for s in [16,32,128,256,512] { try render(s, "icon_\(s)x\(s).png"); try render(s*2, "icon_\(s)x\(s)@2x.png") }
let process=Process(); process.executableURL=URL(fileURLWithPath:"/usr/bin/iconutil");process.arguments=["-c","icns",folder.path,"-o",destination];try process.run();process.waitUntilExit();if process.terminationStatus != 0 { exit(process.terminationStatus) }
