package app.compose.mobile

import android.graphics.Bitmap
import android.net.Uri
import android.widget.EditText
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.espresso.Espresso.onView
import androidx.test.espresso.action.ViewActions.replaceText
import androidx.test.espresso.matcher.ViewMatchers.withId
import androidx.test.espresso.matcher.ViewMatchers.isAssignableFrom
import androidx.test.espresso.ViewAction
import androidx.test.espresso.UiController
import android.view.View
import org.hamcrest.Matcher
import kotlinx.coroutines.*
import org.json.JSONObject
import org.junit.*
import java.io.File

class EditorDeviceTest {
    @get:Rule val rule=createAndroidComposeRule<MainActivity>()
    private val model get()=rule.activity.editorModel
    private fun waitFor(test:()->Boolean){rule.waitUntil(20000,test)}
    private fun act(op:String,vararg fields:Pair<String,Any?>){runBlocking {withContext(Dispatchers.Main){Assert.assertTrue(model.execute(command(op,*fields)))}};rule.waitForIdle()}
    private fun shot(name:String) {
        rule.waitForIdle()
        InstrumentationRegistry.getInstrumentation().uiAutomation.waitForIdle(100,2000)
        val directory=InstrumentationRegistry.getArguments().getString("additionalTestOutputDir")
        val file=File(directory ?: rule.activity.getExternalFilesDir(null)!!.path,"$name.png");file.parentFile?.mkdirs()
        InstrumentationRegistry.getInstrumentation().uiAutomation.takeScreenshot().let{bitmap->file.outputStream().use{bitmap.compress(Bitmap.CompressFormat.PNG,100,it)}}
    }
    @Before fun ready(){waitFor{rule.activity::editorModel.isInitializedForTest()};waitFor{model.state!=null&&model.ready};rule.runOnIdle{model.home=true;model.sheet=null}}
    @Test fun nativeTextSelectionFormattingUndoAndRecreation() {
        rule.onNodeWithTag("Preset Blank").performClick();waitFor{!model.home}
        rule.onNodeWithTag("Text").performClick();waitFor{model.sheet=="Text"&&model.state?.selected?.text!=null}
        val text="Hi 👩‍💻 café Україна\nSecond line"
        rule.onNodeWithTag("Text input").assertExists()
        onView(withId(7007)).perform(replaceText(text))
        waitFor{model.state?.selected?.text?.optString("content")==text}
        onView(withId(7007)).perform(object:ViewAction {
            override fun getConstraints():Matcher<View> = isAssignableFrom(EditText::class.java)
            override fun getDescription()="Select only café using Android text selection"
            override fun perform(ui:UiController,view:View){(view as EditText).setSelection(9,13);ui.loopMainThreadUntilIdle()}
        })
        rule.onNodeWithTag("Bold").performClick()
        waitFor{val spans=model.state?.selected?.text?.getJSONArray("spans");spans!=null&&(0 until spans.length()).any{spans.getJSONObject(it).getJSONObject("style").getInt("weight")==700}}
        val spans=model.state!!.selected!!.text!!.getJSONArray("spans")
        val bold=(0 until spans.length()).map{spans.getJSONObject(it)}.single{it.getJSONObject("style").getInt("weight")==700}
        val bytes=text.toByteArray(Charsets.UTF_8);val range=bold.getJSONObject("range")
        Assert.assertEquals("café",bytes.copyOfRange(range.getInt("start"),range.getInt("end")).toString(Charsets.UTF_8))
        Assert.assertEquals(text,model.state?.selected?.text?.getString("content"))
        val bitmap=model.images["text:${model.state!!.selectedId}"]!!
        val pixels=IntArray(bitmap.width*bitmap.height);bitmap.getPixels(pixels,0,bitmap.width,0,0,bitmap.width,bitmap.height)
        Assert.assertTrue("Emoji must appear in the shared document preview",pixels.any{android.graphics.Color.red(it)>140&&android.graphics.Color.green(it)>70&&android.graphics.Color.blue(it)<100})
        shot("phone-text")
        rule.onNodeWithTag("Done").performClick();waitFor{model.sheet==null}
        rule.onNodeWithTag("Undo").performClick();waitFor{model.state?.selected?.text?.getString("content")==""}
        rule.onNodeWithTag("Redo").performClick();waitFor{model.state?.selected?.text?.getString("content")==text}
        rule.activityRule.scenario.recreate();waitFor{rule.activity.editorModel.state?.selected?.text?.getString("content")==text}
    }
    @Test fun editablePhotoPresetShapesAndNativeFiles() {
        rule.onNodeWithTag("Preset PhotoResume").performScrollTo().performClick();waitFor{!model.home&&model.state?.layers?.any{it.image!=null}==true}
        Assert.assertTrue(model.images.values.any{it.width>500})
        rule.onNodeWithTag("Pages").assertIsDisplayed()
        if(rule.activity.resources.configuration.screenWidthDp>=720)rule.onNodeWithTag("Layer ${model.state!!.layers.first().id}").assertIsDisplayed()
        shot("photo-resume")
        rule.onNodeWithTag("Shapes").performClick();rule.onNodeWithTag("Figure Circle").performClick()
        waitFor{model.sheet==null&&model.state?.selected?.shape=="Circle"}
        val circle=model.state!!.selected!!
        act("frame","frame" to JSONObject().put("x",220).put("y",460).put("width",145).put("height",90))
        Assert.assertEquals(145f,model.state!!.selected!!.height,.01f)
        val file=File(rule.activity.filesDir,"qa-roundtrip.compose")
        act("save","path" to file.path)
        val document=model.state!!.document.toString()
        act("new","preset" to "Blank")
        act("load","path" to file.path)
        Assert.assertEquals(document,model.state!!.document.toString())
        val png=File(rule.activity.filesDir,"qa-output.png");act("png","path" to png.path)
        Assert.assertTrue(png.length()>10000)
        model.select(circle.id);waitFor{model.state?.selectedId==circle.id}
        rule.onNodeWithTag("Properties").performClick();rule.onNodeWithTag("Duplicate").performScrollTo().performClick()
        waitFor{model.state?.selectedId!=circle.id}
        rule.onNodeWithTag("Done").performClick();shot("figures-and-inspector")
        val local=File(rule.activity.cacheDir,"photo.png")
        val image=Bitmap.createBitmap(48,32,Bitmap.Config.ARGB_8888);image.eraseColor(android.graphics.Color.rgb(90,140,190))
        local.outputStream().use{image.compress(Bitmap.CompressFormat.PNG,100,it)}
        val count=model.state!!.layers.size
        rule.runOnIdle{model.importImage(Uri.fromFile(local))}
        waitFor{!model.busy&&model.state!!.layers.size==count+1&&model.state!!.selected?.image!=null}
        Assert.assertEquals(1.5f,model.state!!.selected!!.width/model.state!!.selected!!.height,.01f)
    }
    @Test fun wordModePagesExportsAndNoCanvasFigures() {
        rule.onNodeWithTag("Word mode").performClick();rule.onNodeWithTag("Preset Resume").performClick();waitFor{!model.home&&model.state?.word==true}
        rule.onNodeWithTag("Shapes").assertIsNotEnabled()
        val file=File(rule.activity.filesDir,"qa-resume.docx");act("docx","path" to file.path);Assert.assertTrue(file.length()>1000)
        rule.onNodeWithTag("Pages").performClick();rule.onNodeWithTag("Add page").performClick();waitFor{model.state?.pageIndex==1}
        Assert.assertEquals(2,model.state!!.document.getJSONArray("pages").length())
        rule.onNodeWithTag("Undo").performClick();waitFor{model.state?.document?.getJSONArray("pages")?.length()==1}
        shot("word-resume")
    }
    @Test fun touchMoveResizePanAndPinchKeepEditableGeometry() {
        rule.onNodeWithTag("Preset Blank").performClick();waitFor{!model.home}
        rule.onNodeWithTag("Shapes").performClick();rule.onNodeWithTag("Figure Circle").performClick();waitFor{model.sheet==null&&model.state?.selected?.shape=="Circle"}
        val old=model.state!!.selected!!
        val center=rule.runOnIdle{rule.activity.findViewById<DocumentCanvas>(7008).screenPoint(old.x+old.width/2,old.y+old.height/2)}
        rule.onNodeWithTag("Canvas").performTouchInput{swipe(Offset(center.x,center.y),Offset(center.x+70,center.y+50),500)}
        waitFor{model.state!!.selected!!.x!=old.x}
        val moved=model.state!!.selected!!
        Assert.assertEquals(old.width,moved.width,.01f)
        val corner=rule.runOnIdle{rule.activity.findViewById<DocumentCanvas>(7008).screenPoint(moved.x+moved.width,moved.y+moved.height)}
        rule.onNodeWithTag("Canvas").performTouchInput{swipe(Offset(corner.x,corner.y),Offset(corner.x+50,corner.y+50),500)}
        waitFor{model.state!!.selected!!.width>moved.width}
        Assert.assertEquals(model.state!!.selected!!.width,model.state!!.selected!!.height,.01f)
        val before=model.state!!.document.toString()
        rule.onNodeWithTag("Canvas").performTouchInput{pinch(Offset(400f,600f),Offset(600f,600f),Offset(300f,600f),Offset(700f,600f),500)}
        rule.waitForIdle();Assert.assertEquals(before,model.state!!.document.toString());Assert.assertEquals(old.id,model.state?.selectedId)
        rule.onNodeWithTag("Undo").performClick();waitFor{model.state!!.selected!!.width==moved.width}
        shot("touch-canvas")
    }
    @Test fun liveBackgroundRemovalRetainsPhotoAndUndo() {
        Assume.assumeTrue(InstrumentationRegistry.getArguments().getString("liveBackground")=="true")
        rule.onNodeWithTag("Preset PhotoResume").performScrollTo().performClick();waitFor{!model.home}
        val original=model.state!!.layers.first{it.image!=null}
        model.select(original.id);waitFor{model.state?.selectedId==original.id}
        rule.onNodeWithTag("Properties").performClick()
        rule.onNodeWithTag("Remove background").performScrollTo().performClick()
        rule.waitUntil(210000){!model.busy&&model.state?.selected?.image!=original.image}
        val cutout=model.state!!.selected!!;Assert.assertEquals(original.width,cutout.width,.01f);Assert.assertEquals(original.height,cutout.height,.01f)
        val bitmap=model.images[cutout.image]!!;Assert.assertTrue(bitmap.width<=800&&bitmap.height<=800)
        Assert.assertEquals(0,android.graphics.Color.alpha(bitmap.getPixel(0,0)))
        rule.onNodeWithTag("Done").performClick();shot("live-cutout")
        rule.onNodeWithTag("Undo").performClick();waitFor{model.state?.selected?.image==original.image}
    }
}
// Activity is fully composed before waitUntil enters; checking its model through
// the first frame avoids treating an empty canvas as a successful startup.
private fun kotlin.reflect.KProperty0<EditorModel>.isInitializedForTest() = runCatching{get()}.isSuccess
