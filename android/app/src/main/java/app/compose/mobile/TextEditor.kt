package app.compose.mobile

import android.content.Context
import android.graphics.Typeface
import android.text.*
import android.text.style.*
import android.view.Gravity
import android.view.inputmethod.InputMethodManager
import android.widget.EditText
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.horizontalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import org.json.JSONObject

class RichInput(context:Context):EditText(context) {
    var model:EditorModel?=null
    var updating=false
    private var initializedId:Long?=null
    init {
        id=7007;contentDescription="Document text"
        gravity=Gravity.TOP;setPadding(16,16,16,16)
        setBackgroundColor(android.graphics.Color.rgb(247,248,245))
        setTextColor(android.graphics.Color.rgb(35,38,36))
        inputType=android.text.InputType.TYPE_CLASS_TEXT or android.text.InputType.TYPE_TEXT_FLAG_MULTI_LINE or android.text.InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
        setHorizontallyScrolling(false);minLines=4;maxLines=8;setSelectAllOnFocus(false)
        addTextChangedListener(object:TextWatcher {
            override fun beforeTextChanged(s:CharSequence?,start:Int,count:Int,after:Int){}
            override fun onTextChanged(s:CharSequence?,start:Int,before:Int,count:Int){}
            override fun afterTextChanged(s:Editable?) {
                if(!updating)model?.dispatch(command("text","content" to s.toString()))
            }
        })
    }
    override fun onSelectionChanged(start:Int,end:Int) {
        super.onSelectionChanged(start,end)
        if(!updating){model?.selectionStart=start.coerceAtLeast(0);model?.selectionEnd=end.coerceAtLeast(0)}
    }
    fun update(editor:EditorModel) {
        model=editor
        val layer=editor.state?.selected ?: return
        val rich=layer.text ?: return
        val content=rich.getString("content")
        val first=initializedId!=layer.id
        if(!first&&text.toString()!=content)return
        updating=true
        try {
            val start=if(first)content.length else selectionStart.coerceIn(0,content.length)
            val end=if(first)content.length else selectionEnd.coerceIn(0,content.length)
            if(first||text.toString()!=content)setText(content)
            val editable=text
            for(span in editable.getSpans(0,editable.length,CharacterStyle::class.java))editable.removeSpan(span)
            fun style(value:JSONObject,a:Int,b:Int) {
                if(a>=b)return
                val size=value.getDouble("size").toFloat().coerceIn(10f,36f)
                val family=value.getString("family")
                val weight=value.getDouble("weight").toInt()
                val italic=value.getBoolean("italic")
                val file=when(family){"Noto Serif"->"NotoSerif";"Noto Sans"->"NotoSans";"Noto Sans Mono"->"NotoSansMono";"Anton"->"Anton";"Bebas Neue"->"BebasNeue";else->"Lato"}
                val suffix=when{file=="Lato"&&weight>=850->if(italic)"BlackItalic"else"Black";file=="Lato"&&weight<350->if(italic)"LightItalic"else"Light";weight>=600->if(italic)"BoldItalic"else"Bold";italic->"Italic";else->"Regular"}
                val typeface=runCatching{Typeface.createFromAsset(context.assets,"fonts/$file-$suffix.ttf")}.getOrElse{Typeface.create(if(family.contains("Serif"))"serif"else if(family.contains("Mono"))"monospace"else"sans-serif",if(weight>=600)Typeface.BOLD else Typeface.NORMAL)}
                val flag=Spanned.SPAN_EXCLUSIVE_EXCLUSIVE
                editable.setSpan(AbsoluteSizeSpan(size.toInt(),true),a,b,flag)
                editable.setSpan(object:MetricAffectingSpan(){
                    override fun updateDrawState(paint:TextPaint){paint.typeface=typeface}
                    override fun updateMeasureState(paint:TextPaint){paint.typeface=typeface}
                },a,b,flag)
                editable.setSpan(ForegroundColorSpan(rgba(value.getJSONObject("color"))),a,b,flag)
                if(value.getBoolean("underline"))editable.setSpan(UnderlineSpan(),a,b,flag)
            }
            style(rich.getJSONObject("style"),0,content.length)
            val bytes=content.toByteArray(Charsets.UTF_8)
            fun index(byte:Int)=bytes.copyOfRange(0,byte.coerceIn(0,bytes.size)).toString(Charsets.UTF_8).length
            val spans=rich.getJSONArray("spans")
            for(i in 0 until spans.length()){
                val span=spans.getJSONObject(i);val range=span.getJSONObject("range")
                style(span.getJSONObject("style"),index(range.getInt("start")),index(range.getInt("end")))
            }
            setSelection(start,end);editor.selectionStart=start;editor.selectionEnd=end
            initializedId=layer.id
            if(first)post {requestFocus();(context.getSystemService(Context.INPUT_METHOD_SERVICE)as InputMethodManager).showSoftInput(this,InputMethodManager.SHOW_IMPLICIT)}
        } finally {updating=false}
    }
}
@Composable fun TextEditor(model:EditorModel) {
    var fonts by remember{mutableStateOf(false)}
    var sizes by remember{mutableStateOf(false)}
    Column {
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState())) {
            Action("Bold"){model.format("bold_toggle" to true)}
            Action("Italic"){model.format("italic_toggle" to true)}
            Action("Underline"){model.format("underline_toggle" to true)}
            Box {
                Action("Font"){fonts=true}
                DropdownMenu(expanded=fonts,onDismissRequest={fonts=false}) {
                    listOf("Lato","Noto Sans","Noto Serif","Noto Sans Mono","Anton","Bebas Neue").forEach{family->
                        DropdownMenuItem(text={Text(family)},onClick={fonts=false;model.format("family" to family)})
                    }
                }
            }
            Box {
                Action("Size"){sizes=true}
                DropdownMenu(expanded=sizes,onDismissRequest={sizes=false}) {
                    listOf(12,14,16,18,24,32,48).forEach{size->DropdownMenuItem(text={Text("$size pt")},onClick={sizes=false;model.format("size" to size)})}
                }
            }
        }
        AndroidView(factory={RichInput(it)},update={it.update(model)},modifier=Modifier.fillMaxWidth().heightIn(min=120.dp,max=230.dp).testTag("Text input"))
        Text("Select a passage to format it. With no selection, formatting applies to the whole text box.",Modifier.padding(vertical=10.dp),fontSize=11.sp)
    }
}
