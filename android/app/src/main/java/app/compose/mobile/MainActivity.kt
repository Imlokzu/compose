package app.compose.mobile

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.enableEdgeToEdge
import androidx.activity.compose.*
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.*
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.Alignment
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.viewmodel.compose.viewModel
import org.json.JSONObject

private val Ink=Color(0xFF232624)
private val Muted=Color(0xFF777C76)
private val Blue=Color(0xFF2878ED)
private val Paper=Color(0xFFFFFEFB)
private val Chrome=Color(0xFFF7F8F5)
class MainActivity:ComponentActivity() {
    lateinit var editorModel:EditorModel
    override fun onCreate(savedInstanceState:Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {MaterialTheme(colorScheme=lightColorScheme(primary=Blue,onPrimary=Color.White,primaryContainer=Color(0xFFE6EFFF),onPrimaryContainer=Blue,
            secondary=Blue,secondaryContainer=Color(0xFFE6EFFF),onSecondaryContainer=Blue,surface=Paper,background=Chrome,onSurface=Ink,onBackground=Ink)) {
            val model:EditorModel=viewModel();editorModel=model;ComposeApp(model)
        }}
    }
}
@Composable private fun Mark() {
    Canvas(Modifier.size(30.dp)) {
        val w=size.width;val h=size.height
        drawPath(Path().apply{moveTo(w*.54f,h*.12f);lineTo(w*.85f,h*.24f);lineTo(w*.85f,h*.69f);lineTo(w*.54f,h*.57f);close()},Muted)
        drawPath(Path().apply{moveTo(w*.15f,h*.34f);lineTo(w*.5f,h*.47f);lineTo(w*.5f,h*.93f);lineTo(w*.15f,h*.8f);close()},Ink)
    }
}
@Composable fun Action(label:String,enabled:Boolean=true,selected:Boolean=false,onClick:()->Unit) {
    TextButton(onClick=onClick,enabled=enabled,modifier=Modifier.heightIn(min=48.dp).testTag(label),colors=ButtonDefaults.textButtonColors(contentColor=if(selected)Blue else Ink)) {
        Text(label,fontSize=12.sp,fontWeight=if(selected)FontWeight.SemiBold else FontWeight.Normal,maxLines=1)
    }
}
@OptIn(ExperimentalMaterial3Api::class)
@Composable fun ComposeApp(model:EditorModel) {
    val state=model.state
    var more by remember{mutableStateOf(false)}
    var newWarning by remember{mutableStateOf(false)}
    var canvas by remember{mutableStateOf<DocumentCanvas?>(null)}
    val open=rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()){it?.let{uri->model.fileAction(uri,"open")}}
    val save=rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/octet-stream")){it?.let{uri->model.fileAction(uri,"compose")}}
    val png=rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("image/png")){it?.let{uri->model.fileAction(uri,"png")}}
    val word=rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/vnd.openxmlformats-officedocument.wordprocessingml.document")){it?.let{uri->model.fileAction(uri,"docx")}}
    val photo=rememberLauncherForActivityResult(ActivityResultContracts.GetContent()){it?.let{uri->model.importImage(uri)}}
    BackHandler(enabled=model.sheet!=null||!model.home) {
        if(model.sheet=="Text")model.finishText()else if(model.sheet!=null)model.sheet=null else model.home=true
    }
    if(newWarning)AlertDialog(onDismissRequest={newWarning=false},title={Text("Create another document?")},text={Text("Save a Compose file to keep a separate copy before replacing this local draft.")},
        confirmButton={TextButton(onClick={newWarning=false;model.home=true}){Text("Choose a layout")}},dismissButton={TextButton(onClick={newWarning=false}){Text("Keep editing")}})
    Scaffold(containerColor=Chrome,topBar={Surface(color=Paper) {
        Row(Modifier.fillMaxWidth().statusBarsPadding().height(56.dp).padding(horizontal=12.dp),verticalAlignment=Alignment.CenterVertically) {
            Mark()
            if(model.home){Text("Compose",Modifier.weight(1f).padding(start=8.dp),fontSize=20.sp,fontWeight=FontWeight.SemiBold);Action("Open",!model.busy){open.launch(arrayOf("*/*"))}}
            else {
                Text(state?.title ?: "Compose",Modifier.weight(1f).padding(start=8.dp).clickable{newWarning=true},fontSize=16.sp,fontWeight=FontWeight.Medium,maxLines=1)
                Action("Undo",state?.json?.optBoolean("undo")==true&&!model.busy){model.dispatch(command("undo"))}
                Action("Redo",state?.json?.optBoolean("redo")==true&&!model.busy){model.dispatch(command("redo"))}
                Box {
                    Action("Menu",!model.busy){more=true}
                    DropdownMenu(expanded=more,onDismissRequest={more=false}) {
                        DropdownMenuItem(text={Text("Save Compose file")},onClick={more=false;save.launch("${state?.title ?: "Untitled"}.compose")},modifier=Modifier.testTag("Save file"))
                        DropdownMenuItem(text={Text("Open document")},onClick={more=false;open.launch(arrayOf("*/*"))})
                        DropdownMenuItem(text={Text("Export PNG")},onClick={more=false;png.launch("${state?.title ?: "Untitled"}.png")},modifier=Modifier.testTag("Export PNG"))
                        if(state?.word==true)DropdownMenuItem(text={Text("Export Word (.docx)")},onClick={more=false;word.launch("${state.title}.docx")})
                        DropdownMenuItem(text={Text("Fit page")},onClick={more=false;canvas?.fit()})
                        DropdownMenuItem(text={Text("New document")},onClick={more=false;newWarning=true})
                    }
                }
            }
        }
    }},bottomBar={if(!model.home)Surface(color=Paper) {
        Row(Modifier.fillMaxWidth().navigationBarsPadding().height(60.dp),horizontalArrangement=Arrangement.SpaceEvenly,verticalAlignment=Alignment.CenterVertically) {
            Action("Text",!model.busy){model.insert("Text")}
            Action("Shapes",state?.word==false&&!model.busy){model.sheet="Shapes"}
            Action("Image",!model.busy){photo.launch("image/*")}
            Action("Layers",!model.busy){model.sheet="Layers"}
            Action("Properties",state?.selected!=null&&!model.busy){model.sheet="Properties"}
        }
    }}) {insets->
        Column(Modifier.fillMaxSize().padding(insets)) {
            if(model.busy)LinearProgressIndicator(Modifier.fillMaxWidth())
            if(model.home)Home(model)else if(state!=null) {
                Row(Modifier.fillMaxWidth().height(40.dp).background(Paper).padding(horizontal=12.dp),verticalAlignment=Alignment.CenterVertically) {
                    Text(if(state.word)"Word-compatible"else"Compose canvas",fontSize=11.sp,color=Muted,modifier=Modifier.weight(1f))
                    if(state.selected?.text!=null)Action("Edit text",state.selected?.locked==false&&!model.busy){model.editText()}
                    Action("Pages"){model.sheet="Pages"}
                }
                BoxWithConstraints(Modifier.weight(1f).fillMaxWidth()) {
                    val tablet=maxWidth>=720.dp
                    Row(Modifier.fillMaxSize()) {
                        if(tablet)Surface(Modifier.width(200.dp).fillMaxHeight(),color=Paper){Layers(model)}
                        AndroidView(factory={context->DocumentCanvas(context).also{it.model=model;canvas=it}},modifier=Modifier.weight(1f).fillMaxHeight().clipToBounds().testTag("Canvas"),update={it.model=model;it.update(state,model.images)})
                    }
                }
                if(model.message.isNotEmpty())Text(model.message,Modifier.fillMaxWidth().background(Paper).padding(horizontal=12.dp,vertical=7.dp),fontSize=11.sp,color=Muted,maxLines=2)
            }else Box(Modifier.fillMaxSize(),contentAlignment=Alignment.Center){CircularProgressIndicator()}
        }
    }
    val sheet=model.sheet
    if(sheet!=null)ModalBottomSheet(onDismissRequest={if(sheet=="Text")model.finishText()else model.sheet=null},containerColor=Paper,modifier=Modifier.testTag("Panel")) {
        Column(Modifier.fillMaxWidth().heightIn(max=600.dp).imePadding().padding(horizontal=20.dp)) {
            Row(Modifier.fillMaxWidth().padding(bottom=12.dp),verticalAlignment=Alignment.CenterVertically) {
                Text(if(sheet=="Text")"Edit text"else sheet,Modifier.weight(1f),fontSize=20.sp,fontWeight=FontWeight.SemiBold)
                Action("Done"){if(sheet=="Text")model.finishText()else model.sheet=null}
            }
            when(sheet){"Text"->TextEditor(model);"Shapes"->ShapeList(model);"Properties"->Properties(model);"Layers"->Layers(model);"Pages"->Pages(model)}
            Spacer(Modifier.height(24.dp))
        }
    }
}
@Composable private fun Home(model:EditorModel) {
    var word by remember{mutableStateOf(false)}
    val presets=listOf("Blank" to "Blank page","Resume" to "Résumé","PhotoResume" to "Photo résumé","PhotoCover" to "Photo cover","Report" to "Report","Letter" to "Letter","Portfolio" to "Portfolio","Poster" to "Poster")
    LazyColumn(Modifier.fillMaxSize().testTag("Layouts"),contentPadding=PaddingValues(24.dp),verticalArrangement=Arrangement.spacedBy(10.dp)) {
        item {
            Text("What will you make?",fontSize=30.sp,fontWeight=FontWeight.SemiBold)
            Text("Write like a document. Design like a canvas.",Modifier.padding(top=8.dp,bottom=18.dp),color=Muted,fontSize=15.sp)
            Row(horizontalArrangement=Arrangement.spacedBy(8.dp)) {
                FilterChip(selected=!word,onClick={word=false},label={Text("Compose")},modifier=Modifier.testTag("Compose mode"))
                FilterChip(selected=word,onClick={word=true},label={Text("Word-compatible")},modifier=Modifier.testTag("Word mode"))
            }
            Text(if(word)"Editable text and images · export Word"else"Text, images, figures and free placement",Modifier.padding(vertical=12.dp),fontSize=12.sp,color=Muted)
            if(model.state?.layers?.isNotEmpty()==true)TextButton(onClick={model.home=false},modifier=Modifier.testTag("Continue draft")){Text("Continue local draft →")}
        }
        items(presets){(id,name)->
            val enabled=model.ready&&!model.busy&&(!word||id in listOf("Blank","Resume","Report","Letter"))
            Surface(color=Paper,shape=RoundedCornerShape(8.dp),border=BorderStroke(1.dp,Color(0xFFE3E5E0)),modifier=Modifier.fillMaxWidth().testTag("Preset $id").clickable(enabled=enabled){model.create(id,word)}) {
                Row(Modifier.padding(18.dp),verticalAlignment=Alignment.CenterVertically) {
                    Text(name,Modifier.weight(1f),fontSize=16.sp,fontWeight=FontWeight.Medium,color=if(enabled)Ink else Muted.copy(alpha=.5f))
                    Text(if(!model.ready)"Loading…"else if(enabled)"Create →"else"Compose only",fontSize=12.sp,color=if(enabled)Blue else Muted)
                }
            }
        }
        item{Text("Saved on your device as .compose files.",fontSize=12.sp,color=Muted,modifier=Modifier.padding(top=12.dp))}
    }
}
@Composable private fun ShapeList(model:EditorModel) {
    val shapes=listOf("Rectangle","Circle","Ellipse","Triangle","Diamond","Pentagon","Hexagon","Star","Arrow","Line","Frame")
    Column(Modifier.verticalScroll(rememberScrollState())) {
        for(pair in shapes.chunked(2))Row(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.spacedBy(8.dp)){
            pair.forEach{shape->OutlinedButton(onClick={model.insert(shape)},modifier=Modifier.weight(1f).height(52.dp).testTag("Figure $shape"),shape=RoundedCornerShape(6.dp)){Text(if(shape=="Ellipse")"Oval"else shape)}}
            if(pair.size==1)Spacer(Modifier.weight(1f))
        }
    }
}
@Composable private fun Layers(model:EditorModel) {
    val state=model.state ?: return
    LazyColumn(Modifier.fillMaxWidth().heightIn(max=430.dp)) {
        if(state.layers.isEmpty())item{Text("Your page starts here. Add text, images or a figure.",Modifier.padding(16.dp),color=Muted,fontSize=14.sp)}
        items(if(state.word)state.layers else state.layers.reversed(),key={it.id}){layer->
            Row(Modifier.fillMaxWidth().background(if(layer.id==state.selectedId)Color(0xFFE6EFFF)else Paper).clickable{model.select(layer.id)}.padding(start=(12+layer.depth*12).dp,end=8.dp,top=15.dp,bottom=15.dp).testTag("Layer ${layer.id}"),verticalAlignment=Alignment.CenterVertically) {
                Text(layer.name,Modifier.weight(1f),fontSize=14.sp,maxLines=1);if(layer.locked)Text("Locked",fontSize=11.sp,color=Muted)
            }
        }
    }
}
@Composable private fun Pages(model:EditorModel) {
    val state=model.state ?: return;val pages=state.document.getJSONArray("pages")
    Column(Modifier.verticalScroll(rememberScrollState())) {
        for(i in 0 until pages.length())Action(pages.getJSONObject(i).getString("name"),selected=i==state.pageIndex){model.dispatch(command("page","index" to i)){model.sheet=null}}
        Action("Add page"){model.dispatch(command("add_page")){model.sheet=null}}
    }
}
@Composable private fun Properties(model:EditorModel) {
    val state=model.state ?: return;val layer=state.selected ?: return
    var radius by remember(layer.id,layer.radius){mutableStateOf(layer.radius)}
    val colors=listOf(0xFF232624,0xFF2878ED,0xFF758464,0xFFD3DEC9,0xFFCB8869,0xFFF5DCA5,0xFFD5E5F7,0xFFFFFFFF)
    Column(Modifier.verticalScroll(rememberScrollState()).testTag("Inspector")) {
        Text(layer.name,fontWeight=FontWeight.Medium,fontSize=16.sp)
        Text("${layer.width.toInt()} × ${layer.height.toInt()} pt",fontSize=12.sp,color=Muted,modifier=Modifier.padding(vertical=8.dp))
        if(layer.locked)Text("This layer is locked.",color=Muted)
        if(layer.text!=null)Action("Edit text",!layer.locked){model.editText()}
        else if(!state.word) {
            Text(if(layer.image==null)"Color"else"Appearance",fontSize=12.sp,color=Muted)
            if(layer.image==null)Row(Modifier.padding(vertical=12.dp),horizontalArrangement=Arrangement.spacedBy(8.dp)) {
                colors.forEach{c->Box(Modifier.size(30.dp).background(Color(c),RoundedCornerShape(15.dp)).border(1.dp,Color(0xFFBBC0B7),RoundedCornerShape(15.dp)).clickable(enabled=!layer.locked){
                    model.dispatch(command("style","color" to JSONObject().put("r",(c shr 16).toInt() and 255).put("g",(c shr 8).toInt() and 255).put("b",c.toInt() and 255).put("a",255)))
                })}
            }
            if(layer.shape !in listOf("Circle","Ellipse","Line")) {
                Text("Corners · ${radius.toInt()} pt",fontSize=12.sp)
                Slider(value=radius,onValueChange={radius=it},onValueChangeFinished={model.dispatch(command("style","radius" to radius))},valueRange=0f..minOf(layer.width,layer.height)/2f,enabled=!layer.locked)
            }
            Row(verticalAlignment=Alignment.CenterVertically){Text("Drop shadow",Modifier.weight(1f),fontSize=14.sp);Switch(checked=layer.appearance.getBoolean("shadow"),onCheckedChange={model.dispatch(command("style","shadow" to it))},enabled=!layer.locked)}
            Action("Toggle stroke",!layer.locked){model.dispatch(command("style","stroke" to if(layer.appearance.getDouble("stroke_width")>0)0 else 2))}
        }
        if(layer.image!=null){Action("Remove background",!layer.locked&&!model.busy){model.removeBackground()};Text("Uploads this image for a free cutout up to 800 px. Undo keeps the original.",fontSize=11.sp,color=Muted)}
        Row {Action("Duplicate",!layer.locked){model.dispatch(command("duplicate"))};Action("Delete",!layer.locked){model.dispatch(command("delete")){model.sheet=null}}}
        Row {Action("Forward",!layer.locked){model.dispatch(command("order","forward" to true))};Action("Backward",!layer.locked){model.dispatch(command("order","forward" to false))}}
    }
}
