package app.compose.mobile

import android.content.Context
import android.graphics.*
import android.view.*
import org.json.JSONObject
import kotlin.math.*

fun rgba(value: JSONObject): Int = Color.argb(value.optInt("a",255),value.getInt("r"),value.getInt("g"),value.getInt("b"))
class DocumentCanvas(context: Context) : View(context) {
    var model: EditorModel? = null
    private var state: EditorState? = null
    private var bitmaps: Map<String,Bitmap> = emptyMap()
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG or Paint.FILTER_BITMAP_FLAG)
    private var zoom = 1f
    private var panX = 0f
    private var panY = 0f
    private var initialized = false
    private var pageToken = ""
    private var startX = 0f; private var startY = 0f
    private var lastX = 0f; private var lastY = 0f
    private var original: RectF? = null
    private var preview: RectF? = null
    private var handle = -1
    private var dragged = false
    private var multiTouch = false
    private var touchedLayer: Long? = null
    private val scaleDetector = ScaleGestureDetector(context,object: ScaleGestureDetector.SimpleOnScaleGestureListener() {
        override fun onScale(detector: ScaleGestureDetector): Boolean {
            val old = zoom
            zoom = (zoom * detector.scaleFactor).coerceIn(0.15f,4f)
            panX = detector.focusX - (detector.focusX-panX) * zoom/old
            panY = detector.focusY - (detector.focusY-panY) * zoom/old
            preview=null;original=null;dragged=false;invalidate();return true
        }
    })
    private val detector = GestureDetector(context,object:GestureDetector.SimpleOnGestureListener() {
        override fun onDown(e:MotionEvent)=true
        override fun onDoubleTap(e:MotionEvent):Boolean {model?.editText();return true}
        override fun onLongPress(e:MotionEvent) { if(state?.selected!=null) model?.sheet="Properties" }
    })
    init {
        id=7008
        contentDescription="Document canvas"
        isFocusable=true
        importantForAccessibility=IMPORTANT_FOR_ACCESSIBILITY_YES
    }
    fun update(next:EditorState,images:Map<String,Bitmap>) {
        val token="${next.title}:${next.page.getLong("id")}:${next.pageIndex}"
        if(token!=pageToken){initialized=false;pageToken=token;preview=null;original=null}
        state=next;bitmaps=images
        if(!initialized&&width>0)fit()
        invalidate()
    }
    fun fit() {
        val page=state?.page ?: return
        zoom=min((width-32f*density())/page.getDouble("width").toFloat(),(height-48f*density())/page.getDouble("height").toFloat()).coerceIn(.15f,2f)
        panX=(width-page.getDouble("width").toFloat()*zoom)/2f
        panY=max(24f*density(),(height-page.getDouble("height").toFloat()*zoom)/2f)
        initialized=true;invalidate()
    }
    private fun density()=resources.displayMetrics.density
    fun screenPoint(x:Float,y:Float)=PointF(x*zoom+panX,y*zoom+panY)
    override fun onSizeChanged(w:Int,h:Int,oldw:Int,oldh:Int){super.onSizeChanged(w,h,oldw,oldh);if(oldw!=w)fit()}
    private fun figure(layer:Layer):Path {
        val result=Path();val commands=layer.path ?: return result
        for(i in 0 until commands.length()) {
            val c=commands.getJSONArray(i)
            fun f(n:Int)=c.getDouble(n).toFloat()
            when(c.getString(0)) {
                "M"->result.moveTo(f(1),f(2));"L"->result.lineTo(f(1),f(2))
                "Q"->result.quadTo(f(1),f(2),f(3),f(4))
                "C"->result.cubicTo(f(1),f(2),f(3),f(4),f(5),f(6));"Z"->result.close()
            }
        }
        return result
    }
    private fun bounds(layer:Layer)=RectF(layer.x,layer.y,layer.x+layer.width,layer.y+layer.height)
    private fun handles(r:RectF)=arrayOf(PointF(r.left,r.top),PointF(r.centerX(),r.top),PointF(r.right,r.top),PointF(r.right,r.centerY()),PointF(r.right,r.bottom),PointF(r.centerX(),r.bottom),PointF(r.left,r.bottom),PointF(r.left,r.centerY()))
    override fun onDraw(canvas:Canvas) {
        super.onDraw(canvas);paint.reset();paint.color=Color.rgb(235,237,233);canvas.drawRect(0f,0f,width.toFloat(),height.toFloat(),paint)
        val state=state ?: return
        canvas.save();canvas.translate(panX,panY);canvas.scale(zoom,zoom)
        val page=RectF(0f,0f,state.page.getDouble("width").toFloat(),state.page.getDouble("height").toFloat())
        paint.reset();paint.isAntiAlias=true;paint.color=Color.rgb(255,254,251)
        paint.setShadowLayer(12f,0f,3f,0x17000000);canvas.drawRect(page,paint);paint.clearShadowLayer()
        canvas.save();canvas.clipRect(page)
        for(layer in state.layers.filter{it.visible}) {
            val r=bounds(layer);val displayed=if(layer.id==state.selectedId)preview else null
            canvas.save()
            if(displayed!=null) {
                canvas.translate(displayed.left-r.left,displayed.top-r.top)
                canvas.translate(r.left,r.top);canvas.scale(displayed.width()/r.width(),displayed.height()/r.height());canvas.translate(-r.left,-r.top)
            }
            paint.reset();paint.isAntiAlias=true;paint.isFilterBitmap=true
            val path=if(layer.path!=null)figure(layer)else Path().apply{addRoundRect(r,layer.radius,layer.radius,Path.Direction.CW)}
            if(layer.appearance.getBoolean("shadow"))paint.setShadowLayer(layer.appearance.getDouble("shadow_blur").toFloat()/2,0f,layer.appearance.getDouble("shadow_y").toFloat(),0x24000000)
            paint.color=rgba(layer.node.getJSONObject("fill"));paint.alpha=(Color.alpha(paint.color)*layer.opacity).roundToInt()
            if(layer.kind=="Shape"||layer.kind=="Frame") {
                if(layer.shape=="Line"){paint.style=Paint.Style.STROKE;paint.strokeWidth=layer.appearance.getDouble("stroke_width").toFloat().coerceAtLeast(.5f)}
                canvas.drawPath(path,paint)
            } else layer.assetKey?.let {key -> bitmaps[key]?.let {bitmap ->
                paint.clearShadowLayer();paint.alpha=(255*layer.opacity).roundToInt()
                canvas.save();canvas.clipPath(path)
                val target=RectF(r)
                if(layer.image!=null&&layer.node.getString("image_sizing")=="Scale") {
                    val scale=min(r.width()/bitmap.width,r.height()/bitmap.height)
                    target.set(r.centerX()-bitmap.width*scale/2,r.centerY()-bitmap.height*scale/2,r.centerX()+bitmap.width*scale/2,r.centerY()+bitmap.height*scale/2)
                }
                canvas.drawBitmap(bitmap,null,target,paint);canvas.restore()
            } }
            paint.clearShadowLayer()
            val stroke=layer.appearance.getDouble("stroke_width").toFloat()
            if(stroke>0f&&layer.text==null&&layer.shape!="Line") {
                paint.style=Paint.Style.STROKE;paint.strokeWidth=stroke;paint.color=rgba(layer.appearance.getJSONObject("stroke"));paint.alpha=(Color.alpha(paint.color)*layer.opacity).roundToInt();canvas.drawPath(path,paint)
            }
            canvas.restore()
        }
        canvas.restore()
        state.selected?.let {layer ->
            val r=preview ?: bounds(layer)
            paint.reset();paint.isAntiAlias=true;paint.color=Color.rgb(40,120,237);paint.style=Paint.Style.STROKE;paint.strokeWidth=1.5f*density()/zoom
            canvas.drawRect(r,paint)
            if(!state.word&&!layer.locked&&!layer.node.optBoolean("is_group")) {
                for(p in handles(r)){
                    val side=4f*density()/zoom
                    val box=RectF(p.x-side,p.y-side,p.x+side,p.y+side)
                    paint.style=Paint.Style.FILL;paint.color=Color.WHITE;canvas.drawRoundRect(box,side*.35f,side*.35f,paint)
                    paint.style=Paint.Style.STROKE;paint.color=Color.rgb(40,120,237);canvas.drawRoundRect(box,side*.35f,side*.35f,paint)
                }
            }
        }
        canvas.restore()
    }
    override fun onTouchEvent(event:MotionEvent):Boolean {
        val state=state ?: return true
        if(model?.busy==true)return true
        scaleDetector.onTouchEvent(event);detector.onTouchEvent(event)
        val x=(event.x-panX)/zoom;val y=(event.y-panY)/zoom
        when(event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                parent.requestDisallowInterceptTouchEvent(true)
                startX=x;startY=y;lastX=event.x;lastY=event.y;dragged=false;multiTouch=false;handle=-1;preview=null
                val selected=state.selected
                touchedLayer=null;original=null
                if(selected!=null&&!selected.locked&&!state.word) {
                    val r=bounds(selected)
                    if(!selected.node.optBoolean("is_group")) handle=handles(r).indexOfFirst{hypot((x-it.x)*zoom,(y-it.y)*zoom)<18*density()}
                    if(handle>=0||r.contains(x,y)){touchedLayer=selected.id;original=r}
                }
            }
            MotionEvent.ACTION_POINTER_DOWN -> {preview=null;original=null;touchedLayer=null;dragged=false;multiTouch=true}
            MotionEvent.ACTION_MOVE -> {
                if(event.pointerCount>1) {
                    if(!scaleDetector.isInProgress){panX+=event.x-lastX;panY+=event.y-lastY}
                } else if(!scaleDetector.isInProgress&&original!=null) {
                    val source=original ?: return true
                    val dx=x-startX;val dy=y-startY
                    if(hypot(dx*zoom,dy*zoom)>5*density()||dragged) {
                        dragged=true
                        val result=RectF(source)
                        if(handle<0)result.offset(dx,dy)else {
                            if(handle in listOf(0,6,7))result.left+=dx
                            if(handle in 2..4)result.right+=dx
                            if(handle in 0..2)result.top+=dy
                            if(handle in 4..6)result.bottom+=dy
                            val ratio=when {state.selected?.shape=="Circle"->1f;state.selected?.image!=null&&state.selected?.node?.getString("image_sizing")=="Scale"->source.width()/source.height();else->null}
                            if(ratio!=null) {
                                if(handle in listOf(1,5)) {result.right=result.left+max(8f,result.height())*ratio}
                                else {result.bottom=result.top+max(8f,result.width())/ratio}
                            }
                            if(result.width()<8f)result.right=result.left+8f
                            if(result.height()<8f)result.bottom=result.top+8f
                        }
                        preview=result
                    }
                } else if(event.pointerCount==1&&touchedLayer==null&&hypot(event.x-lastX,event.y-lastY)>2*density()) {
                    panX+=event.x-lastX;panY+=event.y-lastY;dragged=true
                }
                lastX=event.x;lastY=event.y;invalidate()
            }
            MotionEvent.ACTION_UP -> {
                val changed=preview
                if(changed!=null&&dragged) model?.dispatch(command("frame","frame" to JSONObject().put("x",changed.left).put("y",changed.top).put("width",changed.width()).put("height",changed.height())))
                else if(!dragged&&!multiTouch&&!scaleDetector.isInProgress) model?.dispatch(command("hit","x" to x,"y" to y))
                preview=null;original=null;handle=-1;parent.requestDisallowInterceptTouchEvent(false);performClick();invalidate()
            }
            MotionEvent.ACTION_CANCEL -> {preview=null;original=null;handle=-1;parent.requestDisallowInterceptTouchEvent(false);invalidate()}
        }
        return true
    }
    override fun performClick():Boolean{super.performClick();return true}
}
