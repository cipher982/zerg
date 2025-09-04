use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use crate::models::{OpsSeriesPoint, OpsSummary, OpsTopAgent};
use crate::state::APP_STATE;

// ---------------------------------------------------------------
// HUD – compact header widget
// ---------------------------------------------------------------

/// Mount a compact Ops HUD into the header. Admin-gated by attempting a
/// summary fetch; on 403 the HUD stays hidden.
pub fn mount_ops_hud(document: &Document) -> Result<(), JsValue> {
    // Insert container once
    let header = document
        .get_element_by_id("header")
        .ok_or(JsValue::from_str("Header not found"))?;

    let hud = if let Some(el) = document.get_element_by_id("ops-hud") {
        el
    } else {
        let el = document.create_element("div")?;
        el.set_id("ops-hud");
        el.set_class_name("ops-hud");
        el.set_attribute("role", "button")?;
        el.set_attribute("title", "Open Ops Dashboard")?;
        // Click navigates to Ops page
        {
            let doc = document.clone();
            let onclick = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: web_sys::MouseEvent| {
                crate::state::dispatch_global_message(crate::messages::Message::ToggleView(
                    crate::storage::ActiveView::AdminOps,
                ));
            }));
            el.add_event_listener_with_callback("click", onclick.as_ref().unchecked_ref())?;
            onclick.forget();
        }
        header.append_child(&el)?;
        el
    };

    // Initial content skeleton
    hud.set_inner_html("<span class=\"ops-hud-skel\">Ops: …</span>");

    // Kick off initial fetch and polling every ~25s
    wasm_bindgen_futures::spawn_local(async move {
        // Attempt fetch; hide HUD if forbidden
        match crate::network::api_client::ApiClient::get_ops_summary().await {
            Ok(json) => {
                if let Ok(summary) = serde_json::from_str::<OpsSummary>(&json) {
                    crate::state::dispatch_global_message(
                        crate::messages::Message::OpsSummaryLoaded(summary),
                    );
                    // Render immediately
                    if let Some(win) = web_sys::window() {
                        if let Some(doc) = win.document() {
                            let _ = render_ops_hud(&doc);
                        }
                    }
                }
            }
            Err(e) => {
                let err = format!("{:?}", e);
                // Permission denied → non-admin: hide HUD silently
                if err.contains("Permission denied") {
                    if let Some(win) = web_sys::window() {
                        if let Some(doc) = win.document() {
                            if let Some(el) = doc.get_element_by_id("ops-hud") {
                                crate::dom_utils::hide(&el);
                            }
                        }
                    }
                    return; // Don't start poller
                } else {
                    web_sys::console::warn_1(&format!("Ops HUD fetch error: {}", err).into());
                }
            }
        }

        // Poller every ~25s – light touch, summary only
        let window = web_sys::window().unwrap();
        let closure = Closure::<dyn FnMut()>::wrap(Box::new(move || {
            let fut = async {
                if let Ok(json) = crate::network::api_client::ApiClient::get_ops_summary().await {
                    if let Ok(summary) = serde_json::from_str::<OpsSummary>(&json) {
                        crate::state::dispatch_global_message(
                            crate::messages::Message::OpsSummaryLoaded(summary),
                        );
                        if let Some(win) = web_sys::window() {
                            if let Some(doc) = win.document() {
                                let _ = render_ops_hud(&doc);
                            }
                        }
                    }
                }
            };
            wasm_bindgen_futures::spawn_local(fut);
        }));
        let _ = window.set_interval_with_callback_and_timeout_and_arguments(
            closure.as_ref().unchecked_ref(),
            25_000,
            &js_sys::Array::new(),
        );
        closure.forget();
    });

    Ok(())
}

fn pct_color(percent_opt: Option<f64>) -> &'static str {
    let p = percent_opt.unwrap_or(0.0);
    if p >= 100.0 {
        "red"
    } else if p >= 80.0 {
        "amber"
    } else {
        "green"
    }
}

/// Render HUD from state summary
pub fn render_ops_hud(document: &Document) -> Result<(), JsValue> {
    let hud = match document.get_element_by_id("ops-hud") {
        Some(el) => el,
        None => return Ok(()),
    };

    let summary_opt = APP_STATE.with(|s| s.borrow().ops_summary.clone());
    if let Some(sum) = summary_opt {
        let runs = sum.runs_today;
        let cost = sum
            .cost_today_usd
            .map(|v| format!("${:.2}", v))
            .unwrap_or("—".to_string());
        let errs = sum.errors_last_hour;
        let pct = sum
            .budget_global
            .as_ref()
            .and_then(|b| b.percent)
            .unwrap_or(0.0);
        let color = pct_color(sum.budget_global.as_ref().and_then(|b| b.percent));
        let html = format!(
            "<div class=\"ops-hud-inner {color}\"><span>Runs {runs}</span><span>Cost {cost}</span><span>Err {errs}</span><span>Budget {pct:.0}%</span></div>",
            color=color, runs=runs, cost=cost, errs=errs, pct=pct
        );
        hud.set_inner_html(&html);
        crate::dom_utils::show(&hud);
    }

    Ok(())
}

// ---------------------------------------------------------------
// Ops Dashboard – page renderer
// ---------------------------------------------------------------

pub fn render_ops_dashboard(document: &Document) -> Result<(), JsValue> {
    let root = match document.get_element_by_id("ops-dashboard") {
        Some(el) => el,
        None => return Ok(()),
    };

    // Apply TV mode if ?tv=1 is present
    apply_tv_mode(document);

    // If we don't have admin access (determined by hiding HUD earlier), we still try REST once
    let doc_clone = document.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match crate::network::api_client::ApiClient::get_ops_summary().await {
            Ok(json) => match serde_json::from_str::<OpsSummary>(&json) {
                Ok(summary) => {
                    crate::state::dispatch_global_message(
                        crate::messages::Message::OpsSummaryLoaded(summary.clone()),
                    );
                    // Fetch series and top concurrently
                    let runs_fut = crate::network::api_client::ApiClient::get_ops_timeseries(
                        "runs_by_hour",
                        "today",
                    );
                    let errs_fut = crate::network::api_client::ApiClient::get_ops_timeseries(
                        "errors_by_hour",
                        "today",
                    );
                    let top_fut = crate::network::api_client::ApiClient::get_ops_top(
                        "agents",
                        "today",
                        5,
                    );

                    let runs_js = runs_fut.await;
                    let errs_js = errs_fut.await;
                    let top_js = top_fut.await;

                    let mut runs_series: Vec<OpsSeriesPoint> = Vec::new();
                    let mut errs_series: Vec<OpsSeriesPoint> = Vec::new();
                    let mut top_agents: Vec<OpsTopAgent> = summary.top_agents_today.clone();
                    if let Ok(j) = runs_js { let _ = serde_json::from_str::<Vec<OpsSeriesPoint>>(&j).map(|v| runs_series = v); }
                    if let Ok(j) = errs_js { let _ = serde_json::from_str::<Vec<OpsSeriesPoint>>(&j).map(|v| errs_series = v); }
                    if let Ok(j) = top_js { let _ = serde_json::from_str::<Vec<OpsTopAgent>>(&j).map(|v| top_agents = v); }

                    // Render UI from gathered data
                    let _ = build_ops_dashboard_ui(&doc_clone, &summary, &runs_series, &errs_series, &top_agents);

                    // Subscribe to WS ops:events (admin-only; let server enforce)
                    subscribe_ops_events();
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("Bad /ops/summary JSON: {:?}", e).into());
                    show_no_access(&doc_clone, Some("Malformed server response"));
                }
            },
            Err(e) => {
                let err = format!("{:?}", e);
                if err.contains("Permission denied") {
                    show_no_access(&doc_clone, None);
                } else {
                    web_sys::console::error_1(&format!("Ops summary error: {}", err).into());
                    show_no_access(&doc_clone, Some("Failed to load data"));
                }
            }
        }
    });

    Ok(())
}

fn show_no_access(document: &Document, detail: Option<&str>) {
    if let Some(root) = document.get_element_by_id("ops-dashboard") {
        let mut msg = String::from("Ops Dashboard is admin-only.");
        if let Some(d) = detail { msg.push_str(" "); msg.push_str(d); }
        root.set_inner_html(&format!("<div class=\"empty\">{}</div>", msg));
    }
}

fn build_ops_dashboard_ui(
    document: &Document,
    summary: &OpsSummary,
    runs_series: &[OpsSeriesPoint],
    errs_series: &[OpsSeriesPoint],
    top_agents: &[OpsTopAgent],
) -> Result<(), JsValue> {
    let root = document.get_element_by_id("ops-dashboard").unwrap();
    root.set_inner_html("");

    // KPI row ---------------------------------------------------------
    let kpis = document.create_element("div")?;
    kpis.set_class_name("kpi-row");
    let kpi_html = format!(
        "<div class=card><div class=label>Runs Today</div><div class=value>{}</div></div>
         <div class=card><div class=label>Cost Today</div><div class=value>{}</div></div>
         <div class=card><div class=label>Active Users (24h)</div><div class=value>{}</div></div>
         <div class=card><div class=label>Errors (1h)</div><div class=value>{}</div></div>",
        summary.runs_today,
        summary
            .cost_today_usd
            .map(|v| format!("${:.2}", v))
            .unwrap_or("—".to_string()),
        summary.active_users_24h,
        summary.errors_last_hour,
    );
    kpis.set_inner_html(&kpi_html);
    root.append_child(&kpis)?;

    // Budget gauges ---------------------------------------------------
    let gauges = document.create_element("div")?;
    gauges.set_class_name("gauges-row");
    let (user_pct, global_pct) = (
        summary
            .budget_user
            .as_ref()
            .and_then(|b| b.percent)
            .unwrap_or(0.0),
        summary
            .budget_global
            .as_ref()
            .and_then(|b| b.percent)
            .unwrap_or(0.0),
    );
    let user_color = super::ops::pct_color(summary.budget_user.as_ref().and_then(|b| b.percent));
    let global_color = super::ops::pct_color(summary.budget_global.as_ref().and_then(|b| b.percent));
    let gauges_html = format!(
        "<div class=\"gauge {u_c}\"><div>User Budget</div><div class=bar><span style=\"width:{u_pct}%\"></span></div><div class=pct>{u_pct:.0}%</div></div>
         <div class=\"gauge {g_c}\"><div>Global Budget</div><div class=bar><span style=\"width:{g_pct}%\"></span></div><div class=pct>{g_pct:.0}%</div></div>",
        u_c = user_color, g_c = global_color, u_pct = user_pct, g_pct = global_pct
    );
    gauges.set_inner_html(&gauges_html);
    root.append_child(&gauges)?;

    // Sparklines ------------------------------------------------------
    let charts = document.create_element("div")?;
    charts.set_class_name("charts-row");
    charts.append_child(&sparkline(document, "Runs by hour", runs_series)?.into())?;
    charts.append_child(&sparkline(document, "Errors by hour", errs_series)?.into())?;
    root.append_child(&charts)?;

    // Top agents table -----------------------------------------------
    let table = document.create_element("table")?;
    table.set_class_name("top-agents");
    table.set_inner_html(
        "<thead><tr><th>Agent</th><th>Owner</th><th>Runs</th><th>p95 ms</th><th>Cost</th></tr></thead><tbody id=\"ops-top-body\"></tbody>",
    );
    root.append_child(&table)?;
    if let Some(tbody) = table.dyn_ref::<HtmlElement>().unwrap().owner_document().unwrap().get_element_by_id("ops-top-body") {
        let mut rows = String::new();
        for a in top_agents.iter() {
            let cost = a.cost_usd.map(|v| format!("${:.2}", v)).unwrap_or("—".to_string());
            let p95 = a.p95_ms.map(|v| v.to_string()).unwrap_or("-".to_string());
            rows.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                html_escape(&a.name),
                html_escape(a.owner_email.as_deref().unwrap_or("-")),
                a.runs,
                p95,
                cost
            ));
        }
        tbody.set_inner_html(&rows);
    }

    // Live ticker -----------------------------------------------------
    let ticker = document.create_element("div")?;
    ticker.set_id("ops-ticker");
    ticker.set_class_name("ops-ticker");
    root.append_child(&ticker)?;
    render_ticker(document)?;

    Ok(())
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn sparkline(document: &Document, label: &str, series: &[OpsSeriesPoint]) -> Result<Element, JsValue> {
    let wrap = document.create_element("div")?;
    wrap.set_class_name("spark");
    let max = series
        .iter()
        .map(|p| p.value)
        .fold(0.0_f64, |m, v| if v > m { v } else { m })
        .max(1.0);
    // Build simple inline SVG polyline
    let w = 240.0_f64;
    let h = 40.0_f64;
    let n = series.len().max(1) as f64;
    let mut points = String::new();
    for (i, p) in series.iter().enumerate() {
        let x = if n <= 1.0 { 0.0 } else { (i as f64) * (w / (n - 1.0)) };
        let y = if max <= 0.0 { h } else { h - (p.value / max * h) };
        points.push_str(&format!("{:.2},{:.2} ", x, y));
    }
    let svg = format!(
        "<div class=\"label\">{}</div><svg width=\"{}\" height=\"{}\"><polyline fill=\"none\" stroke=\"#4f46e5\" stroke-width=\"2\" points=\"{}\"/></svg>",
        label, w as i32, h as i32, points
    );
    wrap.set_inner_html(&svg);
    Ok(wrap)
}

pub fn render_ticker(document: &Document) -> Result<(), JsValue> {
    let Some(el) = document.get_element_by_id("ops-ticker") else { return Ok(()); };
    // Read events from state
    let events = APP_STATE.with(|s| s.borrow().ops_ticker.clone());
    let mut html = String::new();
    html.push_str("<div class=\"ticker-list\">");
    for ev in events.iter() {
        let class = match ev.kind.as_str() {
            "run_failed" => "red",
            "budget_denied" => "red",
            "run_success" => "green",
            "run_started" => "amber",
            _ => "gray",
        };
        let text = &ev.text;
        html.push_str(&format!("<div class=\"tick {}\">{}</div>", class, html_escape(text)));
    }
    html.push_str("</div>");
    el.set_inner_html(&html);
    Ok(())
}

fn subscribe_ops_events() {
    use std::cell::RefCell;
    use std::rc::Rc;
    use crate::network::topic_manager::ITopicManager;
    let (tm_rc, already) = APP_STATE.with(|s| {
        let st = s.borrow();
        (st.topic_manager.clone() as Rc<RefCell<dyn ITopicManager>>, st.ops_ws_subscribed)
    });
    if already { return; }

    let handler: crate::network::topic_manager::TopicHandler = Rc::new(RefCell::new(move |payload: serde_json::Value| {
        // Parse envelope and only handle ops_event
        if let Ok(envelope) = serde_json::from_value::<crate::generated::ws_messages::Envelope>(payload.clone()) {
            if envelope.message_type == "ops_event" {
                let kind = envelope
                    .data
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("event")
                    .to_string();
                let text = humanize_ops_event(&envelope);
                crate::state::dispatch_global_message(crate::messages::Message::OpsAppendEvent {
                    ts: envelope.ts,
                    kind,
                    text,
                });
            }
        }
    }));

    // Subscribe
    {
        let mut tm = tm_rc.borrow_mut();
        let _ = tm.subscribe("ops:events".to_string(), handler);
    }
    // Mark as subscribed
    APP_STATE.with(|s| s.borrow_mut().ops_ws_subscribed = true);
}

fn humanize_ops_event(env: &crate::generated::ws_messages::Envelope) -> String {
    // Best-effort formatting using typed data
    if let Ok(d) = serde_json::from_value::<crate::generated::ws_messages::OpsEventData>(env.data.clone()) {
        match d.r#type.as_str() {
            "run_started" => format!("Run started – agent {} run {}", d.agent_id.unwrap_or(0), d.run_id.unwrap_or(0)),
            "run_success" => format!("Run success – agent {} in {} ms", d.agent_id.unwrap_or(0), d.duration_ms.unwrap_or(0)),
            "run_failed" => format!("Run failed – agent {}: {}", d.agent_id.unwrap_or(0), d.error.unwrap_or_else(|| "error".into())),
            "agent_created" => format!("Agent created – {}", d.agent_name.unwrap_or_else(|| "(unnamed)".into())),
            "agent_updated" => format!("Agent updated – {}", d.agent_name.unwrap_or_else(|| "(unknown)".into())),
            "thread_message_created" => format!("New message – thread {}", d.thread_id.unwrap_or(0)),
            "budget_denied" => format!("Budget denied – {} {}%", d.scope.unwrap_or_else(|| "user".into()), d.percent.unwrap_or(100.0)),
            other => other.to_string(),
        }
    } else {
        format!("{}", env.message_type)
    }
}

fn apply_tv_mode(document: &Document) {
    if let Some(win) = web_sys::window() {
        if let Ok(search) = win.location().search() {
            if search.contains("tv=1") {
                // Hide non-essential chrome
                if let Some(el) = document.get_element_by_id("global-tabs-container") {
                    crate::dom_utils::hide(&el);
                }
                if let Some(el) = document.get_element_by_id("agent-shelf") {
                    crate::dom_utils::hide(&el);
                }
                if let Some(el) = document.get_element_by_id("header") {
                    // Keep header minimal (hide HUD)
                    if let Some(hud) = document.get_element_by_id("ops-hud") {
                        crate::dom_utils::hide(&hud);
                    }
                }

                // Fallback auto-refresh every 15s regardless of WS state
                let closure = Closure::<dyn FnMut()>::wrap(Box::new(move || {
                    let fut = async {
                        if let Ok(json) = crate::network::api_client::ApiClient::get_ops_summary().await {
                            if let Ok(summary) = serde_json::from_str::<OpsSummary>(&json) {
                                crate::state::dispatch_global_message(
                                    crate::messages::Message::OpsSummaryLoaded(summary.clone()),
                                );
                                if let Some(win) = web_sys::window() {
                                    if let Some(doc) = win.document() {
                                        let _ = crate::components::ops::render_ops_dashboard(&doc);
                                    }
                                }
                            }
                        }
                    };
                    wasm_bindgen_futures::spawn_local(fut);
                }));
                if let Some(win) = web_sys::window() {
                    let _ = win.set_interval_with_callback_and_timeout_and_arguments(
                        closure.as_ref().unchecked_ref(),
                        15_000,
                        &js_sys::Array::new(),
                    );
                }
                closure.forget();
            }
        }
    }
}
