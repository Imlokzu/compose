// Small AppKit adapter. The document editor and page renderer remain Rust.
import AppKit
import PDFKit

struct PrintPage: Decodable { let path: String; let width: Double; let height: Double }
struct PrintDocument: Decodable { let name: String; let pages: [PrintPage] }

final class ComposePrintView: NSView {
    let images: [NSImage]
    let pages: [NSRect]
    override var isFlipped: Bool { true }
    override var isOpaque: Bool { true }
    init(document: PrintDocument) throws {
        var images: [NSImage] = []
        var pages: [NSRect] = []
        var y = 0.0
        for page in document.pages {
            guard page.width.isFinite, page.height.isFinite, page.width > 0,
                  page.height > 0, let image = NSImage(contentsOfFile: page.path) else {
                throw NSError(domain: "ComposePrint", code: 1,
                    userInfo: [NSLocalizedDescriptionKey: "Could not read a prepared print page"])
            }
            images.append(image)
            pages.append(NSRect(x: 0, y: y, width: page.width, height: page.height))
            y += page.height
        }
        self.images = images
        self.pages = pages
        super.init(frame: NSRect(x: 0, y: 0, width: pages.map(\.width).max() ?? 1, height: y))
    }
    required init?(coder: NSCoder) { fatalError("Not used") }
    override func knowsPageRange(_ range: NSRangePointer) -> Bool {
        range.pointee = NSRange(location: 1, length: pages.count)
        return true
    }
    override func rectForPage(_ page: Int) -> NSRect {
        pages[page - 1]
    }
    override func draw(_ dirtyRect: NSRect) {
        for (index, rect) in pages.enumerated() where rect.intersects(dirtyRect) {
            images[index].draw(in: rect, from: .zero, operation: .sourceOver,
                               fraction: 1, respectFlipped: true,
                               hints: [.interpolation: NSImageInterpolation.high])
        }
    }
}

do {
    guard CommandLine.arguments.count >= 2 else { throw NSError(domain: "ComposePrint", code: 2) }
    let document = try JSONDecoder().decode(PrintDocument.self,
        from: Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1])))
    guard !document.pages.isEmpty else { throw NSError(domain: "ComposePrint", code: 3) }
    let app = NSApplication.shared
    app.setActivationPolicy(.regular)
    app.finishLaunching()
    let view = try ComposePrintView(document: document)
    let info = NSPrintInfo(dictionary: [:])
    info.paperSize = NSSize(width: document.pages[0].width, height: document.pages[0].height)
    info.topMargin = 0; info.bottomMargin = 0; info.leftMargin = 0; info.rightMargin = 0
    info.horizontalPagination = .fit; info.verticalPagination = .fit
    info.isHorizontallyCentered = true; info.isVerticallyCentered = true
    let saveIndex = CommandLine.arguments.firstIndex(of: "--save-pdf")
    if let index = saveIndex {
        guard CommandLine.arguments.count > index + 1 else { throw NSError(domain: "ComposePrint", code: 6) }
        info.jobDisposition = .save
        info.dictionary()[NSPrintInfo.AttributeKey.jobSavingURL] = URL(fileURLWithPath: CommandLine.arguments[index + 1])
    }
    // AppKit copies NSPrintInfo here. Configure it before constructing the operation.
    let operation = NSPrintOperation(view: view, printInfo: info)
    operation.jobTitle = document.name
    operation.showsProgressPanel = false
    if let index = saveIndex {
        guard operation.printInfo.jobDisposition == .save,
              let destination = operation.printInfo.dictionary()[NSPrintInfo.AttributeKey.jobSavingURL] as? URL,
              destination.path == URL(fileURLWithPath: CommandLine.arguments[index + 1]).path else {
            throw NSError(domain: "ComposePrint", code: 5,
                userInfo: [NSLocalizedDescriptionKey: "Refusing a print test without PDF-only settings"])
        }
        operation.showsPrintPanel = false
    } else if CommandLine.arguments.contains("--dialog-check") {
        // Exercise the real dialog without any path that can submit a print job.
        app.activate(ignoringOtherApps: true)
        let timer = Timer(timeInterval: 3, repeats: false) { _ in app.abortModal() }
        RunLoop.main.add(timer, forMode: .modalPanel)
        RunLoop.main.add(timer, forMode: .common)
        let result = operation.printPanel.runModal(with: operation.printInfo)
        print("PRINT_DIALOG_CHECK: modal returned \(result)")
        exit(0)
    } else {
        app.activate(ignoringOtherApps: true)
        operation.showsPrintPanel = true
    }
    if operation.run() {
        if let index = saveIndex {
            guard let pdf = PDFDocument(url: URL(fileURLWithPath: CommandLine.arguments[index + 1])),
                  pdf.pageCount == document.pages.count else {
                throw NSError(domain: "ComposePrint", code: 4,
                    userInfo: [NSLocalizedDescriptionKey: "Print output has the wrong page count"])
            }
            print("PRINT_PDF_PASS: \(pdf.pageCount) pages")
        } else { print("PRINT_DONE") }
        exit(0)
    } else { print("PRINT_CANCELLED"); exit(2) }
} catch {
    fputs("Compose print failed: \(error.localizedDescription)\n", stderr)
    exit(1)
}
