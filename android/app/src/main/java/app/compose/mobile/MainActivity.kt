package app.compose.mobile

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.enableEdgeToEdge
import androidx.activity.compose.*
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.*
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.Alignment
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.viewmodel.compose.viewModel

class MainActivity:ComponentActivity() {
    lateinit var editorModel:EditorModel
    override fun onCreate(savedInstanceState:Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {MaterialTheme(colorScheme=lightColorScheme(primary=Blue,onPrimary=Color.White,primaryContainer=Active,onPrimaryContainer=Blue,
            secondary=Blue,secondaryContainer=Active,onSecondaryContainer=Blue,surface=Paper,background=Chrome,onSurface=Ink,onBackground=Ink,
            surfaceContainer=Chrome,surfaceContainerHigh=Chrome,outlineVariant=Edge)) {
            val model:EditorModel=viewModel();editorModel=model;ComposeApp(model)
        }}
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
        Column(Modifier.statusBarsPadding()) {
            Row(Modifier.fillMaxWidth().height(56.dp).padding(horizontal=12.dp),verticalAlignment=Alignment.CenterVertically) {
                Glyph(R.drawable.ic_logo,Modifier.size(28.dp))
                if(model.home){Text("Compose",Modifier.weight(1f).padding(start=8.dp),fontSize=20.sp,fontWeight=FontWeight.SemiBold);Action("Open",!model.busy,icon=R.drawable.ic_open){open.launch(arrayOf("*/*"))}}
                else {
                    Column(Modifier.weight(1f).padding(start=8.dp).clickable{newWarning=true}) {
                        Text(state?.title ?: "Compose",fontSize=14.sp,fontWeight=FontWeight.SemiBold,maxLines=1,overflow=TextOverflow.Ellipsis)
                        Text(if(state?.dirty==true)"Draft saved on device"else"Compose document",fontSize=10.sp,color=Muted)
                    }
                    IconAction("Undo",R.drawable.ic_undo,state?.json?.optBoolean("undo")==true&&!model.busy){model.dispatch(command("undo"))}
                    IconAction("Redo",R.drawable.ic_redo,state?.json?.optBoolean("redo")==true&&!model.busy){model.dispatch(command("redo"))}
                    IconAction("Save file",R.drawable.ic_save,!model.busy){save.launch("${state?.title ?: "Untitled"}.compose")}
                    Box {
                        IconAction("Menu",R.drawable.ic_export,!model.busy){more=true}
                        DropdownMenu(expanded=more,onDismissRequest={more=false}) {
                            DropdownMenuItem(text={Text("Open document")},leadingIcon={Glyph(R.drawable.ic_open)},onClick={more=false;open.launch(arrayOf("*/*"))})
                            DropdownMenuItem(text={Text("Export PNG")},leadingIcon={Glyph(R.drawable.ic_image)},onClick={more=false;png.launch("${state?.title ?: "Untitled"}.png")},modifier=Modifier.testTag("Export PNG"))
                            if(state?.word==true)DropdownMenuItem(text={Text("Export Word (.docx)")},leadingIcon={Glyph(R.drawable.ic_word)},onClick={more=false;word.launch("${state.title}.docx")})
                            HorizontalDivider(color=Edge)
                            DropdownMenuItem(text={Text("Fit page")},leadingIcon={Glyph(R.drawable.ic_preview)},onClick={more=false;canvas?.fit()})
                            DropdownMenuItem(text={Text("New document")},leadingIcon={Glyph(R.drawable.ic_plus)},onClick={more=false;newWarning=true})
                        }
                    }
                }
            }
            HorizontalDivider(color=Edge)
        }
    }},bottomBar={if(!model.home)Surface(color=Paper) {
        Column {HorizontalDivider(color=Edge)
            Row(Modifier.fillMaxWidth().navigationBarsPadding().padding(horizontal=6.dp),horizontalArrangement=Arrangement.spacedBy(2.dp)) {
                Tool("Text",R.drawable.ic_text,Modifier.weight(1f),!model.busy,model.sheet=="Text"){model.insert("Text")}
                Tool("Shapes",R.drawable.ic_shape,Modifier.weight(1f),state?.word==false&&!model.busy,model.sheet=="Shapes"){model.sheet="Shapes"}
                Tool("Image",R.drawable.ic_image,Modifier.weight(1f),!model.busy){photo.launch("image/*")}
                Tool("Layers",R.drawable.ic_layers,Modifier.weight(1f),!model.busy,model.sheet=="Layers"){model.sheet="Layers"}
                Tool("Properties",R.drawable.ic_settings,Modifier.weight(1f),state?.selected!=null&&!model.busy,model.sheet=="Properties"){model.sheet="Properties"}
            }
        }
    }}) {insets->
        Column(Modifier.fillMaxSize().padding(insets)) {
            if(model.busy)LinearProgressIndicator(Modifier.fillMaxWidth())
            if(model.home)Home(model)else if(state!=null) {
                Row(Modifier.fillMaxWidth().height(44.dp).background(Paper).padding(start=16.dp,end=6.dp),verticalAlignment=Alignment.CenterVertically) {
                    Glyph(if(state.word)R.drawable.ic_word else R.drawable.ic_frame,tint=Muted)
                    Text(if(state.word)"Word-compatible"else"Compose canvas",fontSize=11.sp,color=Muted,modifier=Modifier.weight(1f).padding(start=8.dp))
                    if(state.selected?.text!=null)Action("Edit text",state.selected?.locked==false&&!model.busy,icon=R.drawable.ic_text){model.editText()}
                    Action("Pages",icon=R.drawable.ic_page){model.sheet="Pages"}
                }
                HorizontalDivider(color=Edge)
                BoxWithConstraints(Modifier.weight(1f).fillMaxWidth()) {
                    val tablet=maxWidth>=720.dp
                    val wide=maxWidth>=1000.dp
                    Row(Modifier.fillMaxSize()) {
                        if(tablet) {
                            Surface(Modifier.width(210.dp).fillMaxHeight(),color=Paper){Layers(model,fillHeight=true)}
                            VerticalDivider(color=Edge)
                        }
                        AndroidView(factory={context->DocumentCanvas(context).also{it.model=model;canvas=it}},modifier=Modifier.weight(1f).fillMaxHeight().clipToBounds().testTag("Canvas"),update={it.model=model;it.update(state,model.images)})
                        if(wide) {
                            VerticalDivider(color=Edge)
                            Surface(Modifier.width(260.dp).fillMaxHeight(),color=Paper){Column(Modifier.padding(16.dp)){Text("Properties",fontSize=14.sp,fontWeight=FontWeight.SemiBold);if(model.sheet !in listOf("Properties","Text"))Properties(model)}}
                        }
                    }
                }
                if(model.message.isNotEmpty())Text(model.message,Modifier.fillMaxWidth().background(Paper).padding(horizontal=12.dp,vertical=7.dp),fontSize=11.sp,color=Muted,maxLines=2)
            }else Box(Modifier.fillMaxSize(),contentAlignment=Alignment.Center){CircularProgressIndicator()}
        }
    }
    val sheet=model.sheet
    if(sheet!=null)ModalBottomSheet(onDismissRequest={if(sheet=="Text")model.finishText()else model.sheet=null},sheetState=rememberModalBottomSheetState(skipPartiallyExpanded=true),containerColor=Paper,modifier=Modifier.testTag("Panel")) {
        Column(Modifier.fillMaxWidth().heightIn(max=600.dp).imePadding().padding(horizontal=20.dp)) {
            Row(Modifier.fillMaxWidth().padding(bottom=8.dp),verticalAlignment=Alignment.CenterVertically) {
                Glyph(when(sheet){"Text"->R.drawable.ic_text;"Shapes"->R.drawable.ic_shape;"Layers"->R.drawable.ic_layers;"Pages"->R.drawable.ic_page;else->R.drawable.ic_settings})
                Text(if(sheet=="Text")"Edit text"else sheet,Modifier.weight(1f).padding(start=10.dp),fontSize=18.sp,fontWeight=FontWeight.SemiBold)
                Action("Done",icon=R.drawable.ic_check){if(sheet=="Text")model.finishText()else model.sheet=null}
            }
            when(sheet){"Text"->TextEditor(model);"Shapes"->ShapeList(model);"Properties"->Properties(model);"Layers"->Layers(model);"Pages"->Pages(model)}
            Spacer(Modifier.height(24.dp))
        }
    }
}
