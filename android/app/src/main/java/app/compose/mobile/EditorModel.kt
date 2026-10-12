package app.compose.mobile

import android.app.Application
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.net.Uri
import androidx.compose.runtime.*
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.*
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.json.JSONArray
import org.json.JSONObject
import java.io.File

object NativeBridge {
    init { System.loadLibrary("compose_android_bridge") }
    external fun create(): Long
    external fun call(session: Long, command: String): String
    external fun asset(session: Long, key: String): ByteArray
    external fun close(session: Long)
}
fun command(op: String, vararg values: Pair<String, Any?>) = JSONObject().put("op", op).apply {
    values.forEach { (key,value) -> put(key,value ?: JSONObject.NULL) }
}
data class Layer(val node: JSONObject, val bounds: JSONObject, val opacity: Float, val locked: Boolean, val depth: Int, val path: JSONArray?, val visible:Boolean) {
    val id get() = node.getLong("id")
    val name get() = node.getString("name")
    val kind get() = node.get("kind")
    val text get() = (kind as? JSONObject)?.optJSONObject("Text")
    val image get() = (kind as? JSONObject)?.optJSONObject("Image")?.optString("asset")
    val x get() = bounds.getDouble("x").toFloat()
    val y get() = bounds.getDouble("y").toFloat()
    val width get() = bounds.getDouble("width").toFloat()
    val height get() = bounds.getDouble("height").toFloat()
    val shape get() = node.optString("shape", "Rectangle")
    val radius get() = node.optDouble("radius").toFloat()
    val appearance get() = node.getJSONObject("appearance")
    val assetKey get() = image ?: if (text != null) "text:$id" else null
}
data class EditorState(val json: JSONObject) {
    val document get() = json.getJSONObject("document")
    val pageIndex get() = json.getInt("page")
    val page get() = document.getJSONArray("pages").getJSONObject(pageIndex)
    val title get() = document.getString("name")
    val word get() = document.getString("mode") == "Word"
    val selectedId get() = if (json.isNull("selected")) null else json.getLong("selected")
    val layers: List<Layer> get() {
        val scene = json.getJSONArray("scene")
        return (0 until scene.length()).map { scene.getJSONObject(it) }.map {
            Layer(it.getJSONObject("node"), it.getJSONObject("bounds"), it.getDouble("opacity").toFloat(), it.getBoolean("locked"), it.getInt("depth"), it.optJSONArray("path"),it.optBoolean("visible",true))
        }
    }
    val selected get() = layers.find { it.id == selectedId }
    val dirty get() = json.getBoolean("dirty")
}
class EditorModel(application: Application) : AndroidViewModel(application) {
    @Volatile private var session = 0L
    @Volatile private var disposed = false
    private val mutex = Mutex()
    private val recovery = File(application.filesDir, "draft.compose")
    private val context get() = getApplication<Application>()
    var state by mutableStateOf<EditorState?>(null); private set
    var images by mutableStateOf<Map<String, Bitmap>>(emptyMap()); private set
    var previews by mutableStateOf<Map<String, Bitmap>>(emptyMap()); private set
    private val imageVersions = mutableMapOf<String, String>()
    var busy by mutableStateOf(false); private set
    var message by mutableStateOf(""); private set
    var home by mutableStateOf(true)
    var sheet by mutableStateOf<String?>(null)
    var editingId by mutableStateOf<Long?>(null)
    var selectionStart by mutableIntStateOf(0)
    var selectionEnd by mutableIntStateOf(0)
    var ready by mutableStateOf(false); private set
    init {
        viewModelScope.launch {
            mutex.withLock {
                runCatching {
                    withContext(Dispatchers.IO) {
                        session=NativeBridge.create()
                        if(disposed){NativeBridge.close(session);error("Editor closed")}
                        val first = if (recovery.exists()) command("load", "path" to recovery.path) else command("state")
                        fetch(first)
                    }
                }.onSuccess { apply(it); ready = true }
                    .onFailure { message = it.message ?: "Could not open local draft" }
            }
        }
    }
    private data class Update(val state: EditorState, val images: Map<String,Bitmap>)
    private fun fetch(cmd: JSONObject): Update {
        val response = JSONObject(NativeBridge.call(session, cmd.toString()))
        check(response.optBoolean("ok")) { response.optString("error", "Could not complete this action") }
        val next = EditorState(response)
        val nextImages = images.toMutableMap()
        for (layer in next.layers.filter{it.visible}) {
            val key = layer.assetKey ?: continue
            val version = if (layer.text != null) layer.node.toString() else key
            if (imageVersions[key] == version && nextImages[key] != null) continue
            val bytes = NativeBridge.asset(session, key)
            if (bytes.isEmpty()) continue
            val dimensions = BitmapFactory.Options().apply { inJustDecodeBounds = true }
            BitmapFactory.decodeByteArray(bytes, 0, bytes.size, dimensions)
            var sample = 1
            while (dimensions.outWidth / sample > 2048 || dimensions.outHeight / sample > 2048) sample *= 2
            val bitmap = BitmapFactory.decodeByteArray(bytes, 0, bytes.size, BitmapFactory.Options().apply { inSampleSize = sample }) ?: continue
            nextImages[key] = bitmap; imageVersions[key] = version
        }
        val used = next.layers.filter{it.visible}.mapNotNull { it.assetKey }.toSet()
        nextImages.keys.retainAll(used)
        imageVersions.keys.retainAll(used)
        return Update(next, nextImages)
    }
    private fun apply(update: Update) { state = update.state; images = update.images }
    suspend fun execute(cmd: JSONObject, recover: Boolean = true): Boolean = mutex.withLock {
        try {
            val update = withContext(Dispatchers.IO) {
                val result = fetch(cmd)
                if (recover && cmd.getString("op") !in listOf("state", "select", "hit", "page", "begin", "end")) {
                    val saved = JSONObject(NativeBridge.call(session, command("save", "path" to recovery.path, "copy" to true).toString()))
                    check(saved.optBoolean("ok")) { "Local draft could not be saved" }
                }
                result
            }
            apply(update); true
        } catch (error: Exception) { message = error.message ?: "Action failed"; false }
    }
    fun dispatch(cmd: JSONObject, after: (() -> Unit)? = null) {
        viewModelScope.launch { if (execute(cmd)) after?.invoke() }
    }
    fun create(preset: String, word: Boolean) {
        dispatch(command("new", "preset" to preset, "mode" to if (word) "Word" else "Compose")) { home = false; sheet = null }
    }
    suspend fun loadPreviews(word:Boolean) {
        for(preset in listOf("Blank","Resume","PhotoResume","PhotoCover","Report","Letter","Portfolio","Poster")) {
            if(word&&preset !in listOf("Blank","Resume","Report","Letter"))continue
            val key="preset:${if(word)"Word"else"Compose"}:$preset"
            if(previews[key]!=null)continue
            val bitmap=mutex.withLock {withContext(Dispatchers.IO) {
                runCatching {val bytes=NativeBridge.asset(session,key);BitmapFactory.decodeByteArray(bytes,0,bytes.size)}.getOrNull()
            }}
            if(bitmap!=null)previews=previews+(key to bitmap)
            yield()
        }
    }
    fun frame(layer:Layer,field:String,value:Float) {
        val frame=JSONObject().put("x",layer.x).put("y",layer.y).put("width",layer.width).put("height",layer.height).put(field,value)
        dispatch(command("frame","frame" to frame,"axis" to field))
    }
    fun select(id: Long?) { dispatch(command("select", "id" to id)) }
    fun insert(kind: String) {
        dispatch(command("insert", "kind" to kind)) {
            sheet = null
            if (kind == "Text") editText()
        }
    }
    fun editText() {
        if (state?.selected?.text == null || state?.selected?.locked == true) return
        selectionStart = 0; selectionEnd = 0
        dispatch(command("begin")) { editingId = state?.selectedId; sheet = "Text" }
    }
    fun finishText() { editingId = null; dispatch(command("end")); sheet = null }
    fun format(vararg values: Pair<String, Any?>) {
        val current = state?.selected?.text ?: return
        val length = current.getString("content").length
        val start = selectionStart.coerceIn(0,length)
        val end = selectionEnd.coerceIn(0,length)
        dispatch(command("format", "start" to if (start == end) 0 else start, "end" to if (start == end) length else end, *values))
    }
    fun fileAction(uri: Uri, type: String) {
        viewModelScope.launch {
            busy = true
            try {
                val temporary = File(context.cacheDir, "transfer.$type")
                if (type == "open") {
                    withContext(Dispatchers.IO) {
                        context.contentResolver.openInputStream(uri)?.use { input ->
                            temporary.outputStream().use { output ->
                                var total=0L;val buffer=ByteArray(65536)
                                while(true){val count=input.read(buffer);if(count<0)break;total+=count;check(total<=256L*1024*1024){"Document exceeds 256 MB"};output.write(buffer,0,count)}
                            }
                        } ?: error("File could not be opened")
                    }
                    if (execute(command("load", "path" to temporary.path))) { home = false; message = "Document opened" }
                } else {
                    val op = when(type) { "png" -> "png"; "docx" -> "docx"; else -> "save" }
                    if (execute(command(op, "path" to temporary.path,"copy" to true))) {
                        withContext(Dispatchers.IO) {
                            context.contentResolver.openOutputStream(uri, "wt")?.use { output -> temporary.inputStream().use { it.copyTo(output) } }
                                ?: error("Destination could not be written")
                        }
                        if(type=="compose")execute(command("mark_saved"),recover=false)
                        message = if(type == "compose") "Document saved" else "Export complete"
                    }
                }
                temporary.delete()
            } catch(error: Exception) { message = error.message ?: "File action failed" }
            finally { busy = false }
        }
    }
    fun importImage(uri: Uri) {
        viewModelScope.launch {
            busy = true
            val file = File(context.cacheDir,"selected-image")
            try {
                withContext(Dispatchers.IO) {
                    context.contentResolver.openInputStream(uri)?.use { input ->
                        file.outputStream().use { output ->
                            var total=0L;val buffer=ByteArray(65536)
                            while(true) { val n=input.read(buffer);if(n<0)break;total+=n;check(total<=25L*1024*1024){"Image exceeds 25 MB"};output.write(buffer,0,n) }
                        }
                    } ?: error("Photo could not be read")
                }
                if(execute(command("image","path" to file.path))) {sheet=null;message="Image inserted"}
            } catch(error:Exception){message=error.message ?: "Image could not be inserted"}
            finally {file.delete();busy=false}
        }
    }
    fun removeBackground() {
        viewModelScope.launch { busy=true;message="Starting background removal…"
            if(execute(command("remove_background"))) message="Background removed · Undo restores the original"
            busy=false
        }
    }
    override fun onCleared() {
        disposed=true
        val closing=session
        if(closing!=0L)CoroutineScope(Dispatchers.IO).launch { NativeBridge.close(closing) }
        super.onCleared()
    }
}
