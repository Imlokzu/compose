package app.compose.mobile

import androidx.compose.foundation.*
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.*
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.*

@Composable fun Home(model:EditorModel) {
    var word by remember{mutableStateOf(false)}
    LaunchedEffect(word,model.ready){if(model.ready)model.loadPreviews(word)}
    val presets=listOf("Blank" to "Blank page","Resume" to "Résumé","PhotoResume" to "Photo résumé","PhotoCover" to "Photo cover","Report" to "Report","Letter" to "Letter","Portfolio" to "Portfolio","Poster" to "Poster")
    BoxWithConstraints(Modifier.fillMaxSize()) {
        val columns=if(maxWidth>=720.dp)4 else 2
        LazyColumn(Modifier.fillMaxSize().testTag("Layouts"),contentPadding=PaddingValues(24.dp),verticalArrangement=Arrangement.spacedBy(20.dp)) {
            item {
                Text("Make room for ideas.",fontSize=28.sp,fontWeight=FontWeight.SemiBold)
                Text("Write like a document. Design like a canvas.",Modifier.padding(top=8.dp,bottom=20.dp),color=Muted,fontSize=14.sp)
                Row(Modifier.fillMaxWidth().background(Edge.copy(alpha=.65f),RoundedCornerShape(8.dp)).padding(4.dp),horizontalArrangement=Arrangement.spacedBy(4.dp)) {
                    Mode("Compose",R.drawable.ic_frame,!word,Modifier.weight(1f).testTag("Compose mode")){word=false}
                    Mode("Word-compatible",R.drawable.ic_word,word,Modifier.weight(1f).testTag("Word mode")){word=true}
                }
                Text(if(word)"Text and images that export to Word."else"Free placement, figures and your own style.",Modifier.padding(top=12.dp),fontSize=12.sp,color=Muted)
                if(model.state?.layers?.isNotEmpty()==true)Action("Continue draft",icon=R.drawable.ic_undo){model.home=false}
                Text("Choose a starting point",Modifier.padding(top=22.dp),fontSize=13.sp,fontWeight=FontWeight.SemiBold)
            }
            items(presets.chunked(columns)){row->
                Row(horizontalArrangement=Arrangement.spacedBy(16.dp)) {
                    row.forEach{(id,name)->
                        val supported=!word||id in listOf("Blank","Resume","Report","Letter")
                        val preview=model.previews["preset:${if(word&&supported)"Word"else"Compose"}:$id"]
                        val enabled=model.ready&&!model.busy&&supported
                        Column(Modifier.weight(1f).testTag("Preset $id").clip(RoundedCornerShape(8.dp)).clickable(enabled=enabled,role=Role.Button){model.create(id,word)}) {
                            Box(Modifier.fillMaxWidth().aspectRatio(1f).background(Color(0xFFECEDEB),RoundedCornerShape(8.dp)).border(1.dp,Edge,RoundedCornerShape(8.dp)).padding(14.dp),contentAlignment=Alignment.Center) {
                                if(preview!=null)Image(preview.asImageBitmap(),contentDescription="$name preview",modifier=Modifier.fillMaxHeight().aspectRatio(.707f).border(1.dp,Edge),contentScale=ContentScale.Fit,alpha=if(supported)1f else .4f)
                                else if(model.ready)CircularProgressIndicator(Modifier.size(20.dp),strokeWidth=2.dp)
                            }
                            Text(name,Modifier.padding(top=10.dp),fontSize=14.sp,fontWeight=FontWeight.Medium,color=if(supported)Ink else Muted)
                            Text(if(supported)"A4 · Editable layout"else"Compose only",Modifier.padding(top=3.dp),fontSize=11.sp,color=Muted)
                        }
                    }
                }
            }
            item{Text("Your documents stay on your device. Save a .compose file to share or open it on desktop.",fontSize=12.sp,color=Muted,modifier=Modifier.padding(vertical=8.dp))}
        }
    }
}
@Composable private fun Mode(label:String,icon:Int,selected:Boolean,modifier:Modifier,onClick:()->Unit) {
    Row(modifier.heightIn(min=48.dp).clip(RoundedCornerShape(6.dp)).background(if(selected)Paper else Color.Transparent).clickable(role=Role.Button,onClick=onClick).padding(horizontal=8.dp),horizontalArrangement=Arrangement.Center,verticalAlignment=Alignment.CenterVertically) {
        Glyph(icon,tint=if(selected)Blue else Muted);Spacer(Modifier.width(8.dp));Text(label,fontSize=12.sp,fontWeight=FontWeight.Medium,color=if(selected)Ink else Muted)
    }
}
