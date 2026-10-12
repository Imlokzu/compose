package app.compose.mobile

import androidx.compose.foundation.*
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.*
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.*
import org.json.JSONObject

@Composable fun ShapeList(model:EditorModel) {
    val shapes=listOf("Rectangle","Circle","Ellipse","Triangle","Diamond","Pentagon","Hexagon","Star","Arrow","Line","Frame")
    Column(Modifier.verticalScroll(rememberScrollState())) {
        Text("Figures",fontSize=12.sp,color=Muted,modifier=Modifier.padding(bottom=12.dp))
        for(pair in shapes.chunked(2))Row(Modifier.fillMaxWidth().padding(bottom=8.dp),horizontalArrangement=Arrangement.spacedBy(8.dp)){
            pair.forEach{shape->
                OutlinedButton(onClick={model.insert(shape)},modifier=Modifier.weight(1f).height(56.dp).testTag("Figure $shape"),shape=RoundedCornerShape(6.dp),border=BorderStroke(1.dp,Edge),colors=ButtonDefaults.outlinedButtonColors(contentColor=Ink)) {
                    Glyph(figureIcon(shape));Spacer(Modifier.width(10.dp));Text(if(shape=="Ellipse")"Oval"else shape,fontSize=13.sp)
                }
            }
            if(pair.size==1)Spacer(Modifier.weight(1f))
        }
    }
}
@Composable fun Layers(model:EditorModel,fillHeight:Boolean=false) {
    val state=model.state ?: return
    var assets by remember{mutableStateOf(false)}
    var collapsed by remember(state.title,state.page.getLong("id")){mutableStateOf(setOf<Long>())}
    val lookup=state.layers.associateBy{it.id}
    val rows=mutableListOf<Layer>()
    fun collect(nodes:org.json.JSONArray) {
        val indices=(0 until nodes.length()).toList().let{if(state.word)it else it.reversed()}
        for(i in indices) {
            val node=nodes.getJSONObject(i);val id=node.getLong("id")
            lookup[id]?.let{rows.add(it)}
            if(id !in collapsed)collect(node.getJSONArray("children"))
        }
    }
    collect(state.page.getJSONArray("nodes"))
    Column(if(fillHeight)Modifier.fillMaxSize()else Modifier.fillMaxWidth()) {
        Row(Modifier.fillMaxWidth().padding(horizontal=8.dp),horizontalArrangement=Arrangement.SpaceEvenly) {
            Action("Layers tab",selected=!assets,icon=R.drawable.ic_layers,title="Layers"){assets=false}
            Action("Assets",enabled=!state.word,selected=assets,icon=R.drawable.ic_components){assets=true}
        }
        HorizontalDivider(color=Edge)
        if(assets&&!state.word)Components(model)else {
            LazyColumn(Modifier.fillMaxWidth().then(if(fillHeight)Modifier.weight(1f)else Modifier.heightIn(max=430.dp)).testTag("Layer list")) {
                if(state.layers.isEmpty())item{Column(Modifier.padding(20.dp)){Glyph(R.drawable.ic_layers,tint=Muted);Text("Your page starts here.",Modifier.padding(top=12.dp),fontSize=14.sp,fontWeight=FontWeight.Medium);Text("Add text, photos or a figure.",Modifier.padding(top=6.dp),color=Muted,fontSize=12.sp)}}
                items(rows,key={it.id}){layer->
                    val selected=layer.id==state.selectedId
                    Row(Modifier.fillMaxWidth().background(if(selected)Active else Paper).clickable(role=Role.Button){model.select(layer.id)}.padding(start=(12+layer.depth*12).dp,end=12.dp).heightIn(min=48.dp).testTag("Layer ${layer.id}"),verticalAlignment=Alignment.CenterVertically) {
                        if(layer.node.getJSONArray("children").length()>0)IconButton(onClick={collapsed=if(layer.id in collapsed)collapsed-layer.id else collapsed+layer.id},modifier=Modifier.size(48.dp).testTag("Expand layer ${layer.id}")){Glyph(R.drawable.ic_chevron_down,Modifier.rotate(if(layer.id in collapsed)-90f else 0f),Muted)}
                        Glyph(layerIcon(layer),tint=if(selected)Blue else if(layer.visible)Muted else Muted.copy(alpha=.4f));Spacer(Modifier.width(8.dp))
                        Text(layer.name,Modifier.weight(1f),fontSize=12.sp,color=if(!layer.visible)Muted else Ink,maxLines=1,overflow=TextOverflow.Ellipsis)
                        if(layer.locked)Glyph(R.drawable.ic_lock,Modifier.size(14.dp),Muted)
                        if(!layer.visible)Glyph(R.drawable.ic_eye_off,Modifier.size(14.dp),Muted)
                    }
                }
            }
            state.selected?.let {layer->
                HorizontalDivider(color=Edge)
                Row(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.SpaceEvenly) {
                    IconAction(if(layer.node.optBoolean("visible"))"Hide layer"else"Show layer",if(layer.node.optBoolean("visible"))R.drawable.ic_eye else R.drawable.ic_eye_off){model.dispatch(command("visibility","id" to layer.id))}
                    IconAction(if(layer.node.optBoolean("locked"))"Unlock layer"else"Lock layer",if(layer.node.optBoolean("locked"))R.drawable.ic_lock else R.drawable.ic_unlock){model.dispatch(command("lock","id" to layer.id))}
                    IconAction("Move layer up",R.drawable.ic_up,!layer.locked){model.dispatch(command("order","forward" to !state.word))}
                    IconAction("Move layer down",R.drawable.ic_down,!layer.locked){model.dispatch(command("order","forward" to state.word))}
                }
            }
        }
    }
}
@Composable private fun Components(model:EditorModel) {
    val components=listOf(Triple("Title","Title & introduction","A confident opening"),Triple("Callout","Callout","Give an idea some space"),Triple("Quote","Pull quote","Words worth highlighting"),Triple("Stat","Key number","Make a number stand out"),Triple("Card","Feature card","A clear, compact feature"),Triple("Divider","Section divider","A pause between sections"))
    LazyColumn(Modifier.heightIn(max=430.dp)) {
        item{Text("Editable building blocks",fontSize=11.sp,color=Muted,modifier=Modifier.padding(12.dp))}
        items(components){(id,name,description)->
            Row(Modifier.fillMaxWidth().testTag("Component $id").clickable(role=Role.Button){model.dispatch(command("component","kind" to id)){model.sheet=null}}.padding(12.dp),verticalAlignment=Alignment.CenterVertically) {
                Glyph(if(id=="Divider")R.drawable.ic_line else if(id=="Title")R.drawable.ic_text else R.drawable.ic_components,tint=Muted)
                Column(Modifier.weight(1f).padding(start=10.dp)){Text(name,fontSize=13.sp,fontWeight=FontWeight.Medium);Text(description,Modifier.padding(top=4.dp),fontSize=11.sp,color=Muted)}
                Glyph(R.drawable.ic_plus,tint=Muted)
            }
        }
    }
}
@Composable fun Pages(model:EditorModel) {
    val state=model.state ?: return;val pages=state.document.getJSONArray("pages")
    Column(Modifier.verticalScroll(rememberScrollState())) {
        for(i in 0 until pages.length())Action(pages.getJSONObject(i).getString("name"),selected=i==state.pageIndex,icon=R.drawable.ic_page){model.dispatch(command("page","index" to i)){model.sheet=null}}
        Action("Add page",icon=R.drawable.ic_plus){model.dispatch(command("add_page")){model.sheet=null}}
    }
}
@Composable fun Properties(model:EditorModel) {
    val state=model.state ?: return
    val layer=state.selected
    if(layer==null){Column(Modifier.padding(top=32.dp)){Glyph(R.drawable.ic_select,tint=Muted);Text("Select an object",Modifier.padding(top=12.dp),fontSize=14.sp,fontWeight=FontWeight.Medium);Text("Its settings appear here.",Modifier.padding(top=6.dp),fontSize=12.sp,color=Muted)};return}
    var name by remember(layer.id,layer.name){mutableStateOf(layer.name)}
    var radius by remember(layer.id,layer.radius){mutableFloatStateOf(layer.radius)}
    var opacity by remember(layer.id,layer.node.optDouble("opacity")){mutableFloatStateOf(layer.node.optDouble("opacity",1.0).toFloat())}
    val enabled=!layer.locked&&!model.busy
    Column(Modifier.verticalScroll(rememberScrollState()).testTag("Inspector")) {
        Row(Modifier.padding(top=12.dp),verticalAlignment=Alignment.CenterVertically) {Glyph(layerIcon(layer));Text(when{layer.text!=null->"Text";layer.image!=null->"Image";layer.kind=="Frame"->"Frame";else->if(layer.shape=="Ellipse")"Oval"else layer.shape},Modifier.padding(start=8.dp),fontSize=16.sp,fontWeight=FontWeight.SemiBold)}
        if(!state.word)OutlinedTextField(name,{name=it},modifier=Modifier.fillMaxWidth().padding(top=12.dp).testTag("Layer name"),label={Text("Layer name",fontSize=11.sp)},singleLine=true,enabled=enabled,shape=RoundedCornerShape(6.dp),textStyle=LocalTextStyle.current.copy(fontSize=13.sp),trailingIcon={if(name!=layer.name)IconAction("Rename layer",R.drawable.ic_check){model.dispatch(command("style","name" to name))}},colors=OutlinedTextFieldDefaults.colors(unfocusedBorderColor=Edge))
        if(layer.locked)Text("This layer is locked. Unlock it in Layers.",Modifier.padding(top=12.dp),fontSize=12.sp,color=Muted)
        if(layer.text!=null) {
            Action("Edit text",enabled,icon=R.drawable.ic_text){model.editText()}
            val style=layer.text!!.getJSONObject("style")
            Text("${style.getString("family")} · ${style.getDouble("size").toInt()} pt",fontSize=12.sp,color=Muted)
            Text("Select a passage in Edit text to change its font, weight, color or link.",Modifier.padding(top=6.dp),fontSize=11.sp,color=Muted)
        }
        if(!state.word) {
            Section("Position & size",R.drawable.ic_frame)
            Row(horizontalArrangement=Arrangement.spacedBy(8.dp)) {
                NumberField("X",layer.x,Modifier.weight(1f),enabled){model.frame(layer,"x",it)}
                NumberField("Y",layer.y,Modifier.weight(1f),enabled){model.frame(layer,"y",it)}
            }
            Row(Modifier.padding(top=8.dp),horizontalArrangement=Arrangement.spacedBy(8.dp)) {
                NumberField("W",layer.width,Modifier.weight(1f),enabled){if(it>0)model.frame(layer,"width",it)}
                NumberField("H",layer.height,Modifier.weight(1f),enabled){if(it>0)model.frame(layer,"height",it)}
            }
            if(layer.image!=null) {
                Section("Image sizing",R.drawable.ic_ratio)
                Row(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.SpaceEvenly) {
                    Action("Scale",enabled,layer.node.getString("image_sizing")=="Scale",R.drawable.ic_ratio){model.dispatch(command("image_sizing","mode" to "Scale"))}
                    Action("Stretch",enabled,layer.node.getString("image_sizing")=="Stretch",R.drawable.ic_stretch){model.dispatch(command("image_sizing","mode" to "Stretch"))}
                }
                Row(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.SpaceEvenly) {
                    Action("Smaller",enabled,icon=R.drawable.ic_minus){model.dispatch(command("scale","factor" to .8))}
                    Action("Bigger",enabled,icon=R.drawable.ic_plus){model.dispatch(command("scale","factor" to 1.25))}
                }
                Text(if(layer.node.getString("image_sizing")=="Scale")"Keeps original image proportions."else"Width and height change independently.",fontSize=11.sp,color=Muted)
            }
            Section("Appearance",R.drawable.ic_palette)
            if(layer.text==null&&layer.image==null)ColorPalette(rgba(layer.node.getJSONObject("fill")),enabled){model.dispatch(command("style","color" to it))}
            Row(Modifier.padding(top=8.dp),verticalAlignment=Alignment.CenterVertically){Text("Opacity",Modifier.weight(1f),fontSize=12.sp);Text("${(opacity*100).toInt()}%",fontSize=12.sp,color=Muted)}
            Slider(opacity,{opacity=it},onValueChangeFinished={model.dispatch(command("style","opacity" to opacity))},enabled=enabled)
            if(layer.text==null&&layer.shape !in listOf("Circle","Ellipse","Line")) {
                Row(verticalAlignment=Alignment.CenterVertically){Glyph(R.drawable.ic_corners,tint=Muted);Text("Corners",Modifier.weight(1f).padding(start=8.dp),fontSize=12.sp);Text("${radius.toInt()} pt",fontSize=12.sp,color=Muted)}
                Slider(radius,{radius=it},onValueChangeFinished={model.dispatch(command("style","radius" to radius))},valueRange=0f..minOf(layer.width,layer.height)/2f,enabled=enabled)
                Row(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.SpaceEvenly) {
                    listOf("Square" to 0f,"Soft" to 12f,"Round" to minOf(layer.width,layer.height)/2).forEach{(label,value)->Action(label,enabled,layer.radius==value){model.dispatch(command("style","radius" to value))}}
                }
            }
            if(layer.text==null) {
                NumberField("Stroke",layer.appearance.getDouble("stroke_width").toFloat(),Modifier.fillMaxWidth(),enabled){if(it>=0)model.dispatch(command("style","stroke" to it))}
                Row(verticalAlignment=Alignment.CenterVertically){Glyph(R.drawable.ic_shadow,tint=Muted);Text("Drop shadow",Modifier.weight(1f).padding(start=8.dp),fontSize=12.sp);Switch(checked=layer.appearance.getBoolean("shadow"),onCheckedChange={model.dispatch(command("style","shadow" to it))},enabled=enabled)}
            }
        }
        if(layer.image!=null) {
            Section("Image tools",R.drawable.ic_image)
            Action("Remove background",enabled,icon=R.drawable.ic_clear_style){model.removeBackground()}
            Text("Uploads this image for a free cutout up to 800 px. Undo keeps the original.",fontSize=11.sp,color=Muted)
        }
        Section("Arrange",R.drawable.ic_layers)
        if(!state.word)Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState())) {
            listOf("left" to R.drawable.ic_align_left,"center_x" to R.drawable.ic_align_center_x,"right" to R.drawable.ic_align_right,"top" to R.drawable.ic_align_top,"center_y" to R.drawable.ic_align_center_y,"bottom" to R.drawable.ic_align_bottom).forEach{(alignment,icon)->IconAction("Align $alignment to page",icon,enabled){model.dispatch(command("align_page","alignment" to alignment))}}
        }
        Row(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.SpaceEvenly) {
            Action("Forward",enabled,icon=R.drawable.ic_up){model.dispatch(command("order","forward" to true))}
            Action("Backward",enabled,icon=R.drawable.ic_down){model.dispatch(command("order","forward" to false))}
        }
        Row(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.SpaceEvenly) {
            Action("Duplicate",enabled,icon=R.drawable.ic_duplicate){model.dispatch(command("duplicate"))}
            Action("Delete",enabled,icon=R.drawable.ic_trash){model.dispatch(command("delete")){model.sheet=null}}
        }
        Spacer(Modifier.height(16.dp))
    }
}
