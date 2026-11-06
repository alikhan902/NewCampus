package ru.ncfu.ecampus;

import android.app.Activity;
import android.graphics.Color;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.webkit.*;
import android.widget.FrameLayout;
import java.io.*;
import java.util.Collections;
import java.util.HashMap;
import java.util.Map;

public final class MainActivity extends Activity {
    static final String SITE = "https://ecampus.ncfu.ru";
    static final String ROOT = "/__ecampus_mobile__/";
    static final int BLUE = Color.rgb(0,154,208), DARK = Color.rgb(18,18,18);
    private FrameLayout frame;
    private WebView web;

    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        frame = new FrameLayout(this);
        web = new WebView(this);
        frame.addView(web);
        setContentView(frame);
        if (Build.VERSION.SDK_INT >= 30) {
            getWindow().setDecorFitsSystemWindows(false);
            frame.setOnApplyWindowInsetsListener((v,insets) -> {
                android.graphics.Insets edges = insets.getInsets(android.view.WindowInsets.Type.systemBars()
                    | android.view.WindowInsets.Type.displayCutout() | android.view.WindowInsets.Type.ime());
                v.setPadding(edges.left, edges.top, edges.right, edges.bottom);
                return insets;
            });
        }
        color(false);
        web.setBackgroundColor(BLUE);
        WebSettings settings = web.getSettings();
        settings.setJavaScriptEnabled(true);
        settings.setDomStorageEnabled(true);
        settings.setAllowFileAccess(false);
        settings.setAllowContentAccess(false);
        settings.setCacheMode(WebSettings.LOAD_NO_CACHE);
        settings.setMixedContentMode(WebSettings.MIXED_CONTENT_NEVER_ALLOW);
        CookieManager.getInstance().setAcceptCookie(true);
        web.addJavascriptInterface(new Platform(), "Campus");
        web.setWebViewClient(new WebViewClient() {
            @Override public WebResourceResponse shouldInterceptRequest(WebView view, WebResourceRequest req) {
                Uri uri = req.getUrl();
                String path = uri.getPath();
                if ("https".equals(uri.getScheme()) && "ecampus.ncfu.ru".equals(uri.getHost()) && path != null && path.startsWith(ROOT)) {
                    String file = path.substring(ROOT.length());
                    if (!file.matches("[a-zA-Z0-9_-]+\\.(html|js|wasm|svg|css|png)")) return denied();
                    try {
                        Map<String,String> headers = new HashMap<>();
                        headers.put("Cache-Control","no-store");
                        headers.put("Content-Security-Policy","default-src 'none'; script-src " + SITE + ROOT + " 'wasm-unsafe-eval'; style-src " + SITE + ROOT + "; connect-src 'self'; img-src 'self' data:; base-uri 'none'; frame-src 'none'; form-action 'none'");
                        return new WebResourceResponse(mime(file), "UTF-8", 200, "OK", headers, getAssets().open(file));
                    } catch (IOException e) { return denied(); }
                }
                // Only the packaged UI may be a document in this privileged WebView.
                if (req.isForMainFrame() || !"ecampus.ncfu.ru".equals(uri.getHost()) || !"https".equals(uri.getScheme())) return denied();
                return null;
            }
            @Override public boolean shouldOverrideUrlLoading(WebView view, WebResourceRequest req) {
                return true;
            }
        });
        web.loadUrl(SITE + ROOT + "index.html");
    }
    private static WebResourceResponse denied() {
        return new WebResourceResponse("text/plain","UTF-8",404,"Not found",Collections.emptyMap(),new ByteArrayInputStream(new byte[0]));
    }
    private static String mime(String file) {
        if (file.endsWith(".js")) return "application/javascript";
        if (file.endsWith(".css")) return "text/css";
        if (file.endsWith(".svg")) return "image/svg+xml";
        if (file.endsWith(".png")) return "image/png";
        if (file.endsWith(".wasm")) return "application/wasm";
        return "text/html";
    }
    private void color(boolean signed) {
        int color = signed ? DARK : BLUE;
        frame.setBackgroundColor(color);
        getWindow().setStatusBarColor(color);
        getWindow().setNavigationBarColor(color);
    }
    private final class Platform {
        @JavascriptInterface public void theme(boolean signed) { runOnUiThread(() -> color(signed)); }
        @JavascriptInterface public void clearSession() { runOnUiThread(() -> CookieManager.getInstance().removeAllCookies(ignored -> {
            CookieManager.getInstance().flush();
            web.evaluateJavascript("window.campusCookiesCleared?.()", null);
        })); }
        @JavascriptInterface public void close() {runOnUiThread(MainActivity.this::finish);}
    }
    @Override public void onBackPressed() {
        web.evaluateJavascript("typeof campusBack === 'function' && (campusBack(), true)", result -> {
            if (!"true".equals(result)) finish();
        });
    }
    @Override protected void onPause() { super.onPause(); CookieManager.getInstance().flush(); }
    @Override protected void onDestroy() { frame.removeView(web); web.destroy(); super.onDestroy(); }
}
