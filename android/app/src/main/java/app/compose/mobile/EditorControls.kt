package app.compose.mobile

import androidx.compose.foundation.*
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.*
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.*
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.*
import org.json.JSONObject

internal val Ink=Color(0xFF232624)
internal val Muted=Color(0xFF686F68)
internal val Blue=Color(0xFF2878ED)
internal val Paper=Color(0xFFFFFEFB)
internal val Chrome=Color(0xFFF7F8F5)
internal val Edge=Color(0xFFE3E5E0)
internal val Active=Color(0xFFE6EFFF)
internal val Swatches=listOf(0xFF232624,0xFF6D756D,0xFFFFFFFF,0xFF2878ED,0xFF174C96,0xFF758464,0xFFD3DEC9,0xFFCB8869,0xFFF5DCA5,0xFFD5E5F7,0xFF9F6D9C,0xFFEFCBCE)
internal fun colorJson(c:Long)=JSONObject().put("r",(c shr 16).toInt() and 255).put("g",(c shr 8).toInt() and 255).put("b",c.toInt() and 255).put("a",(c shr 24).toInt() and 255)

@Composable fun Glyph(icon:Int,modifier:Modifier=Modifier,tint:Color=Ink) {
    Icon(painterResource(icon),contentDescription=null,tint=if(icon==R.drawable.ic_settings)Color.Unspecified else tint,modifier=modifier.size(20.dp).alpha(tint.alpha))
}
@Composable fun IconAction(label:String,icon:Int,enabled:Boolean=true,selected:Boolean=false,onClick:()->Unit) {
    IconButton(onClick=onClick,enabled=enabled,modifier=Modifier.size(48.dp).clip(RoundedCornerShape(6.dp)).background(if(selected)Active else Color.Transparent).testTag(label).semantics {contentDescription=label}) {
        Glyph(icon,tint=if(!enabled)Muted.copy(alpha=.35f)else if(selected)Blue else Ink)
    }
}
@Composable fun Action(label:String,enabled:Boolean=true,selected:Boolean=false,icon:Int?=null,title:String=label,onClick:()->Unit) {
    TextButton(onClick=onClick,enabled=enabled,modifier=Modifier.heightIn(min=48.dp).testTag(label).semantics{this.selected=selected},shape=RoundedCornerShape(6.dp),colors=ButtonDefaults.textButtonColors(contentColor=if(selected)Blue else Ink)) {
        if(icon!=null){Glyph(icon,tint=if(selected)Blue else if(enabled)Ink else Muted.copy(alpha=.35f));Spacer(Modifier.width(8.dp))}
        Text(title,fontSize=13.sp,fontWeight=if(selected)FontWeight.SemiBold else FontWeight.Normal,maxLines=1)
    }
}
@Composable fun Tool(label:String,icon:Int,modifier:Modifier=Modifier,enabled:Boolean=true,selected:Boolean=false,onClick:()->Unit) {
    Column(modifier.height(64.dp).clip(RoundedCornerShape(6.dp)).background(if(selected)Active else Color.Transparent).clickable(enabled=enabled,role=Role.Button,onClick=onClick).testTag(label).semantics {this.selected=selected}.padding(vertical=9.dp),horizontalAlignment=Alignment.CenterHorizontally,verticalArrangement=Arrangement.spacedBy(5.dp)) {
        val color=if(!enabled)Muted.copy(alpha=.4f)else if(selected)Blue else Ink
        Glyph(icon,tint=color);Text(label,fontSize=10.sp,color=color,maxLines=1)
    }
}
@Composable fun Section(label:String,icon:Int?=null) {
    HorizontalDivider(Modifier.padding(top=16.dp,bottom=10.dp),color=Edge)
    Row(Modifier.padding(bottom=8.dp),verticalAlignment=Alignment.CenterVertically,horizontalArrangement=Arrangement.spacedBy(8.dp)) {
        if(icon!=null)Glyph(icon,tint=Muted)
        Text(label,fontSize=12.sp,fontWeight=FontWeight.SemiBold,color=Muted)
    }
}
@Composable fun NumberField(label:String,value:Float,modifier:Modifier=Modifier,enabled:Boolean=true,onCommit:(Float)->Unit) {
    var input by remember(value){mutableStateOf(if(value==value.toInt().toFloat())value.toInt().toString()else "%.1f".format(java.util.Locale.ROOT,value))}
    val parsed=input.toFloatOrNull()?.takeIf{it.isFinite()}
    val focus=LocalFocusManager.current
    val keyboard=LocalSoftwareKeyboardController.current
    val commit={parsed?.let(onCommit);focus.clearFocus();keyboard?.hide();Unit}
    OutlinedTextField(value=input,onValueChange={input=it},label={Text(label,fontSize=11.sp)},modifier=modifier.testTag("Field $label"),singleLine=true,enabled=enabled,
        textStyle=LocalTextStyle.current.copy(fontSize=14.sp),shape=RoundedCornerShape(6.dp),isError=parsed==null,
        keyboardOptions=KeyboardOptions(keyboardType=KeyboardType.Decimal,imeAction=ImeAction.Done),keyboardActions=KeyboardActions(onDone={commit()}),
        trailingIcon={if(parsed!=null&&parsed!=value)IconAction("Apply $label",R.drawable.ic_check){commit()}},
        colors=OutlinedTextFieldDefaults.colors(unfocusedBorderColor=Edge,focusedBorderColor=Blue))
}
@Composable fun ColorPalette(selected:Int?,enabled:Boolean=true,onColor:(JSONObject)->Unit) {
    Column(Modifier.fillMaxWidth()) {
        Swatches.chunked(4).forEach { row->
            Row(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.SpaceBetween) {
                row.forEach {c->
                    val hex="#%06X".format(c and 0xFFFFFF)
                    Box(Modifier.size(48.dp).testTag("Color $hex").semantics {contentDescription=hex;this.selected=selected==c.toInt()}.clip(RoundedCornerShape(6.dp)).clickable(enabled=enabled,role=Role.Button){onColor(colorJson(c))},contentAlignment=Alignment.Center) {
                        Box(Modifier.size(28.dp).background(Color(c),RoundedCornerShape(5.dp)).border(if(selected==c.toInt())2.dp else 1.dp,if(selected==c.toInt())Blue else Edge,RoundedCornerShape(5.dp)))
                    }
                }
            }
        }
    }
}
internal fun layerIcon(layer:Layer)=when {
    layer.text!=null->R.drawable.ic_text
    layer.image!=null->R.drawable.ic_image
    layer.kind=="Frame"->if(layer.node.optBoolean("is_group"))R.drawable.ic_group else R.drawable.ic_frame
    else->figureIcon(layer.shape)
}
internal fun figureIcon(shape:String)=when(shape) {
    "Circle"->R.drawable.ic_circle;"Ellipse"->R.drawable.ic_oval;"Triangle"->R.drawable.ic_triangle
    "Diamond"->R.drawable.ic_diamond;"Pentagon"->R.drawable.ic_pentagon;"Hexagon"->R.drawable.ic_hexagon
    "Star"->R.drawable.ic_star;"Arrow"->R.drawable.ic_arrow;"Line"->R.drawable.ic_line;"Frame"->R.drawable.ic_frame
    else->R.drawable.ic_shape
}
