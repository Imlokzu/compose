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
import androidx.compose.foundation.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.Alignment
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.*
import androidx.compose.ui.draw.clip
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
                val size=value.getDouble("size").toFloat()
                val family=value.getString("family")
                val weight=value.getDouble("weight").toInt()
                val italic=value.getBoolean("italic")
                val typeface=nativeTypeface(context,family,weight,italic)
                val flag=Spanned.SPAN_EXCLUSIVE_EXCLUSIVE
                editable.setSpan(AbsoluteSizeSpan(size.toInt(),true),a,b,flag)
                editable.setSpan(object:MetricAffectingSpan(){
                    override fun updateDrawState(paint:TextPaint){paint.typeface=typeface;paint.isUnderlineText=value.getBoolean("underline")}
                    override fun updateMeasureState(paint:TextPaint){paint.typeface=typeface}
                },a,b,flag)
                editable.setSpan(ForegroundColorSpan(rgba(value.getJSONObject("color"))),a,b,flag)
                value.optString("link").takeIf{it.isNotEmpty()}?.let{url->editable.setSpan(object:URLSpan(url){override fun updateDrawState(paint:TextPaint){super.updateDrawState(paint);paint.color=rgba(value.getJSONObject("color"))}},a,b,flag)}
            }
            style(rich.getJSONObject("style"),0,content.length)
            val bytes=content.toByteArray(Charsets.UTF_8)
            fun index(byte:Int)=bytes.copyOfRange(0,byte.coerceIn(0,bytes.size)).toString(Charsets.UTF_8).length
            val spans=rich.getJSONArray("spans")
            for(i in 0 until spans.length()){
                val span=spans.getJSONObject(i);val range=span.getJSONObject("range")
                style(span.getJSONObject("style"),index(range.getInt("start")),index(range.getInt("end")))
            }
            gravity=Gravity.TOP or when(rich.optString("align")){"Center"->Gravity.CENTER_HORIZONTAL;"Right"->Gravity.RIGHT;else->Gravity.LEFT}
            justificationMode=if(rich.optString("align")=="Justify")android.text.Layout.JUSTIFICATION_MODE_INTER_WORD else android.text.Layout.JUSTIFICATION_MODE_NONE
            setSelection(start,end);editor.selectionStart=start;editor.selectionEnd=end
            initializedId=layer.id
            if(first)post {requestFocus();(context.getSystemService(Context.INPUT_METHOD_SERVICE)as InputMethodManager).showSoftInput(this,InputMethodManager.SHOW_IMPLICIT)}
        } finally {updating=false}
    }
}
private val faceCache=mutableMapOf<String,Typeface>()
internal fun nativeTypeface(context:Context,family:String,weight:Int=400,italic:Boolean=false):Typeface {
    val file=when(family){"Noto Serif"->"NotoSerif";"Noto Sans"->"NotoSans";"Noto Sans Mono"->"NotoSansMono";"Anton"->"Anton";"Bebas Neue"->"BebasNeue";else->"Lato"}
    val canItalic=file in listOf("Lato","NotoSans","NotoSerif")&&italic
    val suffix=when{file in listOf("Anton","BebasNeue")->"Regular";file=="Lato"&&weight>=850->if(canItalic)"BlackItalic"else"Black";file=="Lato"&&weight<350->if(canItalic)"LightItalic"else"Light";weight>=600->if(canItalic)"BoldItalic"else"Bold";canItalic->"Italic";else->"Regular"}
    val key="$file-$suffix"
    return faceCache.getOrPut(key){Typeface.createFromAsset(context.assets,"fonts/$key.ttf")}
}
private fun currentStyle(model:EditorModel):JSONObject? {
    val rich=model.state?.selected?.text ?: return null
    val text=rich.getString("content")
    val start=if(model.selectionStart==model.selectionEnd)0 else minOf(model.selectionStart,model.selectionEnd).coerceIn(0,text.length)
    val byte=text.substring(0,start).toByteArray(Charsets.UTF_8).size
    val spans=rich.getJSONArray("spans")
    return (0 until spans.length()).map{spans.getJSONObject(it)}.find{it.getJSONObject("range").let{r->byte>=r.getInt("start")&&byte<r.getInt("end")}}?.getJSONObject("style") ?: rich.getJSONObject("style")
}
private fun selectedStyles(model:EditorModel):List<JSONObject> {
    val rich=model.state?.selected?.text ?: return emptyList()
    val content=rich.getString("content")
    val whole=model.selectionStart==model.selectionEnd
    val start=if(whole)0 else minOf(model.selectionStart,model.selectionEnd).coerceIn(0,content.length)
    val end=if(whole)content.length else maxOf(model.selectionStart,model.selectionEnd).coerceIn(0,content.length)
    val a=content.substring(0,start).toByteArray().size;val b=content.substring(0,end).toByteArray().size
    val spans=rich.getJSONArray("spans");val bounds=mutableSetOf(a,b)
    for(i in 0 until spans.length())spans.getJSONObject(i).getJSONObject("range").let{r->listOf(r.getInt("start"),r.getInt("end")).filter{it>a&&it<b}.forEach{bounds.add(it)}}
    return bounds.sorted().zipWithNext().map{(index,_)->
        (0 until spans.length()).map{spans.getJSONObject(it)}.find{it.getJSONObject("range").let{r->index>=r.getInt("start")&&index<r.getInt("end")}}?.getJSONObject("style") ?: rich.getJSONObject("style")
    }.ifEmpty{listOf(rich.getJSONObject("style"))}
}
@Composable private fun StyleButton(label:String,letter:String,selected:Boolean,enabled:Boolean=true,onClick:()->Unit) {
    TextButton(onClick,Modifier.size(48.dp).testTag(label).semantics{contentDescription=label;this.selected=selected},enabled=enabled,shape=RoundedCornerShape(6.dp),colors=ButtonDefaults.textButtonColors(containerColor=if(selected)Active else Color.Transparent,contentColor=if(selected)Blue else Ink),contentPadding=PaddingValues(0.dp)) {
        Text(letter,fontSize=17.sp,fontWeight=if(label=="Bold")FontWeight.Bold else FontWeight.Medium,fontStyle=if(label=="Italic")FontStyle.Italic else FontStyle.Normal,textDecoration=if(label=="Underline")TextDecoration.Underline else TextDecoration.None)
    }
}
@Composable fun TextEditor(model:EditorModel) {
    var fonts by remember{mutableStateOf(false)}
    var sizes by remember{mutableStateOf(false)}
    var weights by remember{mutableStateOf(false)}
    var colors by remember{mutableStateOf(false)}
    var linkDialog by remember{mutableStateOf(false)}
    var url by remember{mutableStateOf("")}
    val rich=model.state?.selected?.text ?: return
    val style=currentStyle(model) ?: return
    val styles=selectedStyles(model)
    fun mixed(field:String)=styles.map{it.opt(field)?.toString()}.distinct().size>1
    val family=style.getString("family")
    val weight=style.getDouble("weight").toInt()
    val size=style.getDouble("size").toInt()
    val supportedWeights=when(family){"Lato"->listOf(300 to "Light",400 to "Regular",700 to "Bold",900 to "Black");"Anton","Bebas Neue"->listOf(400 to "Regular");else->listOf(400 to "Regular",700 to "Bold")}
    val canItalic=family in listOf("Lato","Noto Sans","Noto Serif")
    val context=LocalContext.current
    if(linkDialog)AlertDialog(onDismissRequest={linkDialog=false},title={Text("Link selected text")},text={OutlinedTextField(url,{url=it},label={Text("https://…")},singleLine=true,modifier=Modifier.testTag("Link URL"))},confirmButton={Action("Apply link"){model.format("link" to url.trim());linkDialog=false}},dismissButton={Action("Remove link"){model.format("link" to "");linkDialog=false}})
    Column {
        Row(Modifier.fillMaxWidth().background(Chrome,RoundedCornerShape(6.dp)),verticalAlignment=Alignment.CenterVertically) {
            Box(Modifier.weight(1f)) {
                TextButton(onClick={fonts=true},modifier=Modifier.fillMaxWidth().height(48.dp).testTag("Font"),shape=RoundedCornerShape(6.dp)) {
                    Glyph(R.drawable.ic_typeface,tint=Muted);Text(if(mixed("family"))"Mixed fonts"else family,Modifier.weight(1f).padding(start=8.dp),fontSize=14.sp,color=Ink,maxLines=1);Glyph(R.drawable.ic_chevron_down,tint=Muted)
                }
                DropdownMenu(expanded=fonts,onDismissRequest={fonts=false}) {
                    listOf("Lato","Noto Sans","Noto Serif","Noto Sans Mono","Anton","Bebas Neue").forEach{choice->
                        val preview=remember(choice){FontFamily(nativeTypeface(context,choice))}
                        DropdownMenuItem(text={Text(choice,fontFamily=preview,fontSize=17.sp)},leadingIcon={if(family==choice)Glyph(R.drawable.ic_check,tint=Blue)},onClick={fonts=false;model.format("family" to choice)},modifier=Modifier.testTag("Font $choice"))
                    }
                }
            }
            Box {
                TextButton(onClick={sizes=true},modifier=Modifier.height(48.dp).testTag("Size")){Text(if(mixed("size"))"Mixed"else"$size pt",color=Ink,fontSize=13.sp);Spacer(Modifier.width(6.dp));Glyph(R.drawable.ic_chevron_down,tint=Muted)}
                DropdownMenu(expanded=sizes,onDismissRequest={sizes=false}) {
                    listOf(10,12,14,16,18,20,24,32,48,64,72,96).forEach{choice->DropdownMenuItem(text={Text("$choice pt")},onClick={sizes=false;model.format("size" to choice)},modifier=Modifier.testTag("Size $choice"))}
                }
            }
        }
        Row(Modifier.fillMaxWidth().padding(vertical=4.dp).horizontalScroll(rememberScrollState()),verticalAlignment=Alignment.CenterVertically) {
            StyleButton("Bold","B",styles.all{it.getDouble("weight")>=600},supportedWeights.size>1){model.format("bold_toggle" to true)}
            StyleButton("Italic","I",styles.all{it.getBoolean("italic")},canItalic){model.format("italic_toggle" to true)}
            StyleButton("Underline","U",styles.all{it.getBoolean("underline")}){model.format("underline_toggle" to true)}
            Box {
                TextButton(onClick={weights=true},modifier=Modifier.height(48.dp).testTag("Weight")){Text(if(mixed("weight"))"Mixed"else supportedWeights.find{it.first==weight}?.second ?: "Regular",fontSize=12.sp,color=Ink);Glyph(R.drawable.ic_chevron_down,tint=Muted)}
                DropdownMenu(expanded=weights,onDismissRequest={weights=false}) {
                    supportedWeights.forEach{(value,label)->DropdownMenuItem(text={Text(label,fontFamily=FontFamily(nativeTypeface(context,family,value)))},onClick={weights=false;model.format("weight" to value)},modifier=Modifier.testTag("Weight $label"))}
                }
            }
            Box {
                IconAction("Text color",R.drawable.ic_palette,selected=colors){colors=true}
                DropdownMenu(expanded=colors,onDismissRequest={colors=false}) {Box(Modifier.width(280.dp).padding(12.dp)){ColorPalette(rgba(style.getJSONObject("color"))){model.format("color" to it);colors=false}}}
            }
            IconAction("Hyperlink",R.drawable.ic_link,selected=style.optString("link").isNotEmpty()){url=style.optString("link");linkDialog=true}
        }
        AndroidView(factory={RichInput(it)},update={it.update(model)},modifier=Modifier.fillMaxWidth().heightIn(min=100.dp,max=190.dp).clip(RoundedCornerShape(6.dp)).testTag("Text input"))
        Row(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.Center) {
            listOf("Left" to R.drawable.ic_left,"Center" to R.drawable.ic_center,"Right" to R.drawable.ic_right).forEach{(alignment,icon)->IconAction("Text align $alignment",icon,selected=rich.optString("align")==alignment){model.format("alignment" to alignment)}}
        }
        Text(if(model.selectionStart!=model.selectionEnd)"Formatting selected text"else"Select a passage to format it, or style the whole text box.",Modifier.padding(vertical=4.dp),fontSize=11.sp,color=Muted)
    }
}
