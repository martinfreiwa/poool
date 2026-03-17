(function (e) {
  ((e.ng ??= {}), (e.ng.common ??= {}), (e.ng.common.locales ??= {}));
  let t = void 0;
  function a(n) {
    let c = n;
    return 5;
  }
  e.ng.common.locales.id = [
    "id",
    [["AM", "PM"], t, t],
    t,
    [
      ["M", "S", "S", "R", "K", "J", "S"],
      ["Min", "Sen", "Sel", "Rab", "Kam", "Jum", "Sab"],
      ["Minggu", "Senin", "Selasa", "Rabu", "Kamis", "Jumat", "Sabtu"],
      ["Min", "Sen", "Sel", "Rab", "Kam", "Jum", "Sab"],
    ],
    t,
    [
      ["J", "F", "M", "A", "M", "J", "J", "A", "S", "O", "N", "D"],
      [
        "Jan",
        "Feb",
        "Mar",
        "Apr",
        "Mei",
        "Jun",
        "Jul",
        "Agu",
        "Sep",
        "Okt",
        "Nov",
        "Des",
      ],
      [
        "Januari",
        "Februari",
        "Maret",
        "April",
        "Mei",
        "Juni",
        "Juli",
        "Agustus",
        "September",
        "Oktober",
        "November",
        "Desember",
      ],
    ],
    t,
    [["SM", "M"], t, ["Sebelum Masehi", "Masehi"]],
    0,
    [6, 0],
    ["dd/MM/yy", "d MMM y", "d MMMM y", "EEEE, dd MMMM y"],
    ["HH.mm", "HH.mm.ss", "HH.mm.ss z", "HH.mm.ss zzzz"],
    ["{1} {0}", t, t, t],
    [",", ".", ";", "%", "+", "-", "E", "\xD7", "\u2030", "\u221E", "NaN", "."],
    ["#,##0.###", "#,##0%", "\xA4#,##0.00", "#E0"],
    "IDR",
    "Rp",
    "Rupiah Indonesia",
    {
      AUD: ["AU$", "$"],
      BYN: [t, "\u0440."],
      IDR: ["Rp"],
      INR: ["Rs", "\u20B9"],
      JPY: ["JP\xA5", "\xA5"],
      PHP: [t, "\u20B1"],
      THB: ["\u0E3F"],
      TWD: ["NT$"],
      USD: ["US$", "$"],
      XXX: [],
    },
    "ltr",
    a,
    [
      [["tengah malam", "tengah hari", "pagi", "siang", "sore", "malam"], t, t],
      t,
      [
        "00:00",
        "12:00",
        ["00:00", "10:00"],
        ["10:00", "15:00"],
        ["15:00", "18:00"],
        ["18:00", "24:00"],
      ],
    ],
  ];
})(globalThis);
var ce = globalThis;
function te(e) {
  return (ce.__Zone_symbol_prefix || "__zone_symbol__") + e;
}
function dt() {
  let e = ce.performance;
  function t(L) {
    e && e.mark && e.mark(L);
  }
  function a(L, s) {
    e && e.measure && e.measure(L, s);
  }
  t("Zone");
  class n {
    static {
      this.__symbol__ = te;
    }
    static assertZonePatched() {
      if (ce.Promise !== C.ZoneAwarePromise)
        throw new Error(
          "Zone.js has detected that ZoneAwarePromise `(window|global).Promise` has been overwritten.\nMost likely cause is that a Promise polyfill has been loaded after Zone.js (Polyfilling Promise api is not necessary when zone.js is loaded. If you must load one, do so before loading zone.js.)",
        );
    }
    static get root() {
      let s = n.current;
      for (; s.parent; ) s = s.parent;
      return s;
    }
    static get current() {
      return b.zone;
    }
    static get currentTask() {
      return D;
    }
    static __load_patch(s, i, o = !1) {
      if (C.hasOwnProperty(s)) {
        let g = ce[te("forceDuplicateZoneCheck")] === !0;
        if (!o && g) throw Error("Already loaded patch: " + s);
      } else if (!ce["__Zone_disable_" + s]) {
        let g = "Zone:" + s;
        (t(g), (C[s] = i(ce, n, P)), a(g, g));
      }
    }
    get parent() {
      return this._parent;
    }
    get name() {
      return this._name;
    }
    constructor(s, i) {
      ((this._parent = s),
        (this._name = i ? i.name || "unnamed" : "<root>"),
        (this._properties = (i && i.properties) || {}),
        (this._zoneDelegate = new f(
          this,
          this._parent && this._parent._zoneDelegate,
          i,
        )));
    }
    get(s) {
      let i = this.getZoneWith(s);
      if (i) return i._properties[s];
    }
    getZoneWith(s) {
      let i = this;
      for (; i; ) {
        if (i._properties.hasOwnProperty(s)) return i;
        i = i._parent;
      }
      return null;
    }
    fork(s) {
      if (!s) throw new Error("ZoneSpec required!");
      return this._zoneDelegate.fork(this, s);
    }
    wrap(s, i) {
      if (typeof s != "function")
        throw new Error("Expecting function got: " + s);
      let o = this._zoneDelegate.intercept(this, s, i),
        g = this;
      return function () {
        return g.runGuarded(o, this, arguments, i);
      };
    }
    run(s, i, o, g) {
      b = { parent: b, zone: this };
      try {
        return this._zoneDelegate.invoke(this, s, i, o, g);
      } finally {
        b = b.parent;
      }
    }
    runGuarded(s, i = null, o, g) {
      b = { parent: b, zone: this };
      try {
        try {
          return this._zoneDelegate.invoke(this, s, i, o, g);
        } catch (F) {
          if (this._zoneDelegate.handleError(this, F)) throw F;
        }
      } finally {
        b = b.parent;
      }
    }
    runTask(s, i, o) {
      if (s.zone != this)
        throw new Error(
          "A task can only be run in the zone of creation! (Creation: " +
            (s.zone || $).name +
            "; Execution: " +
            this.name +
            ")",
        );
      let g = s,
        { type: F, data: { isPeriodic: ee = !1, isRefreshable: A = !1 } = {} } =
          s;
      if (s.state === X && (F === U || F === m)) return;
      let he = s.state != Z;
      he && g._transitionTo(Z, d);
      let _e = D;
      ((D = g), (b = { parent: b, zone: this }));
      try {
        F == m && s.data && !ee && !A && (s.cancelFn = void 0);
        try {
          return this._zoneDelegate.invokeTask(this, g, i, o);
        } catch (Q) {
          if (this._zoneDelegate.handleError(this, Q)) throw Q;
        }
      } finally {
        let Q = s.state;
        if (Q !== X && Q !== q)
          if (F == U || ee || (A && Q === k)) he && g._transitionTo(d, Z, k);
          else {
            let Ee = g._zoneDelegates;
            (this._updateTaskCount(g, -1),
              he && g._transitionTo(X, Z, X),
              A && (g._zoneDelegates = Ee));
          }
        ((b = b.parent), (D = _e));
      }
    }
    scheduleTask(s) {
      if (s.zone && s.zone !== this) {
        let o = this;
        for (; o; ) {
          if (o === s.zone)
            throw Error(
              `can not reschedule task to ${this.name} which is descendants of the original zone ${s.zone.name}`,
            );
          o = o.parent;
        }
      }
      s._transitionTo(k, X);
      let i = [];
      ((s._zoneDelegates = i), (s._zone = this));
      try {
        s = this._zoneDelegate.scheduleTask(this, s);
      } catch (o) {
        throw (
          s._transitionTo(q, k, X),
          this._zoneDelegate.handleError(this, o),
          o
        );
      }
      return (
        s._zoneDelegates === i && this._updateTaskCount(s, 1),
        s.state == k && s._transitionTo(d, k),
        s
      );
    }
    scheduleMicroTask(s, i, o, g) {
      return this.scheduleTask(new E(G, s, i, o, g, void 0));
    }
    scheduleMacroTask(s, i, o, g, F) {
      return this.scheduleTask(new E(m, s, i, o, g, F));
    }
    scheduleEventTask(s, i, o, g, F) {
      return this.scheduleTask(new E(U, s, i, o, g, F));
    }
    cancelTask(s) {
      if (s.zone != this)
        throw new Error(
          "A task can only be cancelled in the zone of creation! (Creation: " +
            (s.zone || $).name +
            "; Execution: " +
            this.name +
            ")",
        );
      if (!(s.state !== d && s.state !== Z)) {
        s._transitionTo(x, d, Z);
        try {
          this._zoneDelegate.cancelTask(this, s);
        } catch (i) {
          throw (
            s._transitionTo(q, x),
            this._zoneDelegate.handleError(this, i),
            i
          );
        }
        return (
          this._updateTaskCount(s, -1),
          s._transitionTo(X, x),
          (s.runCount = -1),
          s
        );
      }
    }
    _updateTaskCount(s, i) {
      let o = s._zoneDelegates;
      i == -1 && (s._zoneDelegates = null);
      for (let g = 0; g < o.length; g++) o[g]._updateTaskCount(s.type, i);
    }
  }
  let c = {
    name: "",
    onHasTask: (L, s, i, o) => L.hasTask(i, o),
    onScheduleTask: (L, s, i, o) => L.scheduleTask(i, o),
    onInvokeTask: (L, s, i, o, g, F) => L.invokeTask(i, o, g, F),
    onCancelTask: (L, s, i, o) => L.cancelTask(i, o),
  };
  class f {
    get zone() {
      return this._zone;
    }
    constructor(s, i, o) {
      ((this._taskCounts = { microTask: 0, macroTask: 0, eventTask: 0 }),
        (this._zone = s),
        (this._parentDelegate = i),
        (this._forkZS = o && (o && o.onFork ? o : i._forkZS)),
        (this._forkDlgt = o && (o.onFork ? i : i._forkDlgt)),
        (this._forkCurrZone = o && (o.onFork ? this._zone : i._forkCurrZone)),
        (this._interceptZS = o && (o.onIntercept ? o : i._interceptZS)),
        (this._interceptDlgt = o && (o.onIntercept ? i : i._interceptDlgt)),
        (this._interceptCurrZone =
          o && (o.onIntercept ? this._zone : i._interceptCurrZone)),
        (this._invokeZS = o && (o.onInvoke ? o : i._invokeZS)),
        (this._invokeDlgt = o && (o.onInvoke ? i : i._invokeDlgt)),
        (this._invokeCurrZone =
          o && (o.onInvoke ? this._zone : i._invokeCurrZone)),
        (this._handleErrorZS = o && (o.onHandleError ? o : i._handleErrorZS)),
        (this._handleErrorDlgt =
          o && (o.onHandleError ? i : i._handleErrorDlgt)),
        (this._handleErrorCurrZone =
          o && (o.onHandleError ? this._zone : i._handleErrorCurrZone)),
        (this._scheduleTaskZS =
          o && (o.onScheduleTask ? o : i._scheduleTaskZS)),
        (this._scheduleTaskDlgt =
          o && (o.onScheduleTask ? i : i._scheduleTaskDlgt)),
        (this._scheduleTaskCurrZone =
          o && (o.onScheduleTask ? this._zone : i._scheduleTaskCurrZone)),
        (this._invokeTaskZS = o && (o.onInvokeTask ? o : i._invokeTaskZS)),
        (this._invokeTaskDlgt = o && (o.onInvokeTask ? i : i._invokeTaskDlgt)),
        (this._invokeTaskCurrZone =
          o && (o.onInvokeTask ? this._zone : i._invokeTaskCurrZone)),
        (this._cancelTaskZS = o && (o.onCancelTask ? o : i._cancelTaskZS)),
        (this._cancelTaskDlgt = o && (o.onCancelTask ? i : i._cancelTaskDlgt)),
        (this._cancelTaskCurrZone =
          o && (o.onCancelTask ? this._zone : i._cancelTaskCurrZone)),
        (this._hasTaskZS = null),
        (this._hasTaskDlgt = null),
        (this._hasTaskDlgtOwner = null),
        (this._hasTaskCurrZone = null));
      let g = o && o.onHasTask,
        F = i && i._hasTaskZS;
      (g || F) &&
        ((this._hasTaskZS = g ? o : c),
        (this._hasTaskDlgt = i),
        (this._hasTaskDlgtOwner = this),
        (this._hasTaskCurrZone = this._zone),
        o.onScheduleTask ||
          ((this._scheduleTaskZS = c),
          (this._scheduleTaskDlgt = i),
          (this._scheduleTaskCurrZone = this._zone)),
        o.onInvokeTask ||
          ((this._invokeTaskZS = c),
          (this._invokeTaskDlgt = i),
          (this._invokeTaskCurrZone = this._zone)),
        o.onCancelTask ||
          ((this._cancelTaskZS = c),
          (this._cancelTaskDlgt = i),
          (this._cancelTaskCurrZone = this._zone)));
    }
    fork(s, i) {
      return this._forkZS
        ? this._forkZS.onFork(this._forkDlgt, this.zone, s, i)
        : new n(s, i);
    }
    intercept(s, i, o) {
      return this._interceptZS
        ? this._interceptZS.onIntercept(
            this._interceptDlgt,
            this._interceptCurrZone,
            s,
            i,
            o,
          )
        : i;
    }
    invoke(s, i, o, g, F) {
      return this._invokeZS
        ? this._invokeZS.onInvoke(
            this._invokeDlgt,
            this._invokeCurrZone,
            s,
            i,
            o,
            g,
            F,
          )
        : i.apply(o, g);
    }
    handleError(s, i) {
      return this._handleErrorZS
        ? this._handleErrorZS.onHandleError(
            this._handleErrorDlgt,
            this._handleErrorCurrZone,
            s,
            i,
          )
        : !0;
    }
    scheduleTask(s, i) {
      let o = i;
      if (this._scheduleTaskZS)
        (this._hasTaskZS && o._zoneDelegates.push(this._hasTaskDlgtOwner),
          (o = this._scheduleTaskZS.onScheduleTask(
            this._scheduleTaskDlgt,
            this._scheduleTaskCurrZone,
            s,
            i,
          )),
          o || (o = i));
      else if (i.scheduleFn) i.scheduleFn(i);
      else if (i.type == G) B(i);
      else throw new Error("Task is missing scheduleFn.");
      return o;
    }
    invokeTask(s, i, o, g) {
      return this._invokeTaskZS
        ? this._invokeTaskZS.onInvokeTask(
            this._invokeTaskDlgt,
            this._invokeTaskCurrZone,
            s,
            i,
            o,
            g,
          )
        : i.callback.apply(o, g);
    }
    cancelTask(s, i) {
      let o;
      if (this._cancelTaskZS)
        o = this._cancelTaskZS.onCancelTask(
          this._cancelTaskDlgt,
          this._cancelTaskCurrZone,
          s,
          i,
        );
      else {
        if (!i.cancelFn) throw Error("Task is not cancelable");
        o = i.cancelFn(i);
      }
      return o;
    }
    hasTask(s, i) {
      try {
        this._hasTaskZS &&
          this._hasTaskZS.onHasTask(
            this._hasTaskDlgt,
            this._hasTaskCurrZone,
            s,
            i,
          );
      } catch (o) {
        this.handleError(s, o);
      }
    }
    _updateTaskCount(s, i) {
      let o = this._taskCounts,
        g = o[s],
        F = (o[s] = g + i);
      if (F < 0) throw new Error("More tasks executed then were scheduled.");
      if (g == 0 || F == 0) {
        let ee = {
          microTask: o.microTask > 0,
          macroTask: o.macroTask > 0,
          eventTask: o.eventTask > 0,
          change: s,
        };
        this.hasTask(this._zone, ee);
      }
    }
  }
  class E {
    constructor(s, i, o, g, F, ee) {
      if (
        ((this._zone = null),
        (this.runCount = 0),
        (this._zoneDelegates = null),
        (this._state = "notScheduled"),
        (this.type = s),
        (this.source = i),
        (this.data = g),
        (this.scheduleFn = F),
        (this.cancelFn = ee),
        !o)
      )
        throw new Error("callback is not defined");
      this.callback = o;
      let A = this;
      s === U && g && g.useG
        ? (this.invoke = E.invokeTask)
        : (this.invoke = function () {
            return E.invokeTask.call(ce, A, this, arguments);
          });
    }
    static invokeTask(s, i, o) {
      (s || (s = this), K++);
      try {
        return (s.runCount++, s.zone.runTask(s, i, o));
      } finally {
        (K == 1 && Y(), K--);
      }
    }
    get zone() {
      return this._zone;
    }
    get state() {
      return this._state;
    }
    cancelScheduleRequest() {
      this._transitionTo(X, k);
    }
    _transitionTo(s, i, o) {
      if (this._state === i || this._state === o)
        ((this._state = s), s == X && (this._zoneDelegates = null));
      else
        throw new Error(
          `${this.type} '${this.source}': can not transition to '${s}', expecting state '${i}'${o ? " or '" + o + "'" : ""}, was '${this._state}'.`,
        );
    }
    toString() {
      return this.data && typeof this.data.handleId < "u"
        ? this.data.handleId.toString()
        : Object.prototype.toString.call(this);
    }
    toJSON() {
      return {
        type: this.type,
        state: this.state,
        source: this.source,
        zone: this.zone.name,
        runCount: this.runCount,
      };
    }
  }
  let T = te("setTimeout"),
    p = te("Promise"),
    w = te("then"),
    _ = [],
    R = !1,
    I;
  function j(L) {
    if ((I || (ce[p] && (I = ce[p].resolve(0))), I)) {
      let s = I[w];
      (s || (s = I.then), s.call(I, L));
    } else ce[T](L, 0);
  }
  function B(L) {
    (K === 0 && _.length === 0 && j(Y), L && _.push(L));
  }
  function Y() {
    if (!R) {
      for (R = !0; _.length; ) {
        let L = _;
        _ = [];
        for (let s = 0; s < L.length; s++) {
          let i = L[s];
          try {
            i.zone.runTask(i, null, null);
          } catch (o) {
            P.onUnhandledError(o);
          }
        }
      }
      (P.microtaskDrainDone(), (R = !1));
    }
  }
  let $ = { name: "NO ZONE" },
    X = "notScheduled",
    k = "scheduling",
    d = "scheduled",
    Z = "running",
    x = "canceling",
    q = "unknown",
    G = "microTask",
    m = "macroTask",
    U = "eventTask",
    C = {},
    P = {
      symbol: te,
      currentZoneFrame: () => b,
      onUnhandledError: W,
      microtaskDrainDone: W,
      scheduleMicroTask: B,
      showUncaughtError: () => !n[te("ignoreConsoleErrorUncaughtError")],
      patchEventTarget: () => [],
      patchOnProperties: W,
      patchMethod: () => W,
      bindArguments: () => [],
      patchThen: () => W,
      patchMacroTask: () => W,
      patchEventPrototype: () => W,
      isIEOrEdge: () => !1,
      getGlobalObjects: () => {},
      ObjectDefineProperty: () => W,
      ObjectGetOwnPropertyDescriptor: () => {},
      ObjectCreate: () => {},
      ArraySlice: () => [],
      patchClass: () => W,
      wrapWithCurrentZone: () => W,
      filterProperties: () => [],
      attachOriginToPatched: () => W,
      _redefineProperty: () => W,
      patchCallbacks: () => W,
      nativeScheduleMicroTask: j,
    },
    b = { parent: null, zone: new n(null, null) },
    D = null,
    K = 0;
  function W() {}
  return (a("Zone", "Zone"), n);
}
function _t() {
  let e = globalThis,
    t = e[te("forceDuplicateZoneCheck")] === !0;
  if (e.Zone && (t || typeof e.Zone.__symbol__ != "function"))
    throw new Error("Zone already loaded.");
  return ((e.Zone ??= dt()), e.Zone);
}
var be = Object.getOwnPropertyDescriptor,
  Ze = Object.defineProperty,
  He = Object.getPrototypeOf,
  Et = Object.create,
  Tt = Array.prototype.slice,
  je = "addEventListener",
  xe = "removeEventListener",
  Me = te(je),
  Ie = te(xe),
  ae = "true",
  le = "false",
  Re = te("");
function Fe(e, t) {
  return Zone.current.wrap(e, t);
}
function Ge(e, t, a, n, c) {
  return Zone.current.scheduleMacroTask(e, t, a, n, c);
}
var H = te,
  De = typeof window < "u",
  pe = De ? window : void 0,
  J = (De && pe) || globalThis,
  gt = "removeAttribute";
function Ve(e, t) {
  for (let a = e.length - 1; a >= 0; a--)
    typeof e[a] == "function" && (e[a] = Fe(e[a], t + "_" + a));
  return e;
}
function mt(e, t) {
  let a = e.constructor.name;
  for (let n = 0; n < t.length; n++) {
    let c = t[n],
      f = e[c];
    if (f) {
      let E = be(e, c);
      if (!tt(E)) continue;
      e[c] = ((T) => {
        let p = function () {
          return T.apply(this, Ve(arguments, a + "." + c));
        };
        return (fe(p, T), p);
      })(f);
    }
  }
}
function tt(e) {
  return e
    ? e.writable === !1
      ? !1
      : !(typeof e.get == "function" && typeof e.set > "u")
    : !0;
}
var nt = typeof WorkerGlobalScope < "u" && self instanceof WorkerGlobalScope,
  Ce =
    !("nw" in J) &&
    typeof J.process < "u" &&
    J.process.toString() === "[object process]",
  ze = !Ce && !nt && !!(De && pe.HTMLElement),
  rt =
    typeof J.process < "u" &&
    J.process.toString() === "[object process]" &&
    !nt &&
    !!(De && pe.HTMLElement),
  we = {},
  yt = H("enable_beforeunload"),
  Je = function (e) {
    if (((e = e || J.event), !e)) return;
    let t = we[e.type];
    t || (t = we[e.type] = H("ON_PROPERTY" + e.type));
    let a = this || e.target || J,
      n = a[t],
      c;
    if (ze && a === pe && e.type === "error") {
      let f = e;
      ((c =
        n && n.call(this, f.message, f.filename, f.lineno, f.colno, f.error)),
        c === !0 && e.preventDefault());
    } else
      ((c = n && n.apply(this, arguments)),
        e.type === "beforeunload" && J[yt] && typeof c == "string"
          ? (e.returnValue = c)
          : c != null && !c && e.preventDefault());
    return c;
  };
function Ye(e, t, a) {
  let n = be(e, t);
  if (
    (!n && a && be(a, t) && (n = { enumerable: !0, configurable: !0 }),
    !n || !n.configurable)
  )
    return;
  let c = H("on" + t + "patched");
  if (e.hasOwnProperty(c) && e[c]) return;
  (delete n.writable, delete n.value);
  let f = n.get,
    E = n.set,
    T = t.slice(2),
    p = we[T];
  (p || (p = we[T] = H("ON_PROPERTY" + T)),
    (n.set = function (w) {
      let _ = this;
      if ((!_ && e === J && (_ = J), !_)) return;
      (typeof _[p] == "function" && _.removeEventListener(T, Je),
        E && E.call(_, null),
        (_[p] = w),
        typeof w == "function" && _.addEventListener(T, Je, !1));
    }),
    (n.get = function () {
      let w = this;
      if ((!w && e === J && (w = J), !w)) return null;
      let _ = w[p];
      if (_) return _;
      if (f) {
        let R = f.call(this);
        if (R)
          return (
            n.set.call(this, R),
            typeof w[gt] == "function" && w.removeAttribute(t),
            R
          );
      }
      return null;
    }),
    Ze(e, t, n),
    (e[c] = !0));
}
function ot(e, t, a) {
  if (t) for (let n = 0; n < t.length; n++) Ye(e, "on" + t[n], a);
  else {
    let n = [];
    for (let c in e) c.slice(0, 2) == "on" && n.push(c);
    for (let c = 0; c < n.length; c++) Ye(e, n[c], a);
  }
}
var oe = H("originalInstance");
function ve(e) {
  let t = J[e];
  if (!t) return;
  ((J[H(e)] = t),
    (J[e] = function () {
      let c = Ve(arguments, e);
      switch (c.length) {
        case 0:
          this[oe] = new t();
          break;
        case 1:
          this[oe] = new t(c[0]);
          break;
        case 2:
          this[oe] = new t(c[0], c[1]);
          break;
        case 3:
          this[oe] = new t(c[0], c[1], c[2]);
          break;
        case 4:
          this[oe] = new t(c[0], c[1], c[2], c[3]);
          break;
        default:
          throw new Error("Arg list too long.");
      }
    }),
    fe(J[e], t));
  let a = new t(function () {}),
    n;
  for (n in a)
    (e === "XMLHttpRequest" && n === "responseBlob") ||
      (function (c) {
        typeof a[c] == "function"
          ? (J[e].prototype[c] = function () {
              return this[oe][c].apply(this[oe], arguments);
            })
          : Ze(J[e].prototype, c, {
              set: function (f) {
                typeof f == "function"
                  ? ((this[oe][c] = Fe(f, e + "." + c)), fe(this[oe][c], f))
                  : (this[oe][c] = f);
              },
              get: function () {
                return this[oe][c];
              },
            });
      })(n);
  for (n in t) n !== "prototype" && t.hasOwnProperty(n) && (J[e][n] = t[n]);
}
function ue(e, t, a) {
  let n = e;
  for (; n && !n.hasOwnProperty(t); ) n = He(n);
  !n && e[t] && (n = e);
  let c = H(t),
    f = null;
  if (n && (!(f = n[c]) || !n.hasOwnProperty(c))) {
    f = n[c] = n[t];
    let E = n && be(n, t);
    if (tt(E)) {
      let T = a(f, c, t);
      ((n[t] = function () {
        return T(this, arguments);
      }),
        fe(n[t], f));
    }
  }
  return f;
}
function pt(e, t, a) {
  let n = null;
  function c(f) {
    let E = f.data;
    return (
      (E.args[E.cbIdx] = function () {
        f.invoke.apply(this, arguments);
      }),
      n.apply(E.target, E.args),
      f
    );
  }
  n = ue(
    e,
    t,
    (f) =>
      function (E, T) {
        let p = a(E, T);
        return p.cbIdx >= 0 && typeof T[p.cbIdx] == "function"
          ? Ge(p.name, T[p.cbIdx], p, c)
          : f.apply(E, T);
      },
  );
}
function fe(e, t) {
  e[H("OriginalDelegate")] = t;
}
var $e = !1,
  Le = !1;
function kt() {
  try {
    let e = pe.navigator.userAgent;
    if (e.indexOf("MSIE ") !== -1 || e.indexOf("Trident/") !== -1) return !0;
  } catch {}
  return !1;
}
function vt() {
  if ($e) return Le;
  $e = !0;
  try {
    let e = pe.navigator.userAgent;
    (e.indexOf("MSIE ") !== -1 ||
      e.indexOf("Trident/") !== -1 ||
      e.indexOf("Edge/") !== -1) &&
      (Le = !0);
  } catch {}
  return Le;
}
function Ke(e) {
  return typeof e == "function";
}
function Qe(e) {
  return typeof e == "number";
}
var ye = !1;
if (typeof window < "u")
  try {
    let e = Object.defineProperty({}, "passive", {
      get: function () {
        ye = !0;
      },
    });
    (window.addEventListener("test", e, e),
      window.removeEventListener("test", e, e));
  } catch {
    ye = !1;
  }
var bt = { useG: !0 },
  ne = {},
  st = {},
  it = new RegExp("^" + Re + "(\\w+)(true|false)$"),
  ct = H("propagationStopped");
function at(e, t) {
  let a = (t ? t(e) : e) + le,
    n = (t ? t(e) : e) + ae,
    c = Re + a,
    f = Re + n;
  ((ne[e] = {}), (ne[e][le] = c), (ne[e][ae] = f));
}
function Rt(e, t, a, n) {
  let c = (n && n.add) || je,
    f = (n && n.rm) || xe,
    E = (n && n.listeners) || "eventListeners",
    T = (n && n.rmAll) || "removeAllListeners",
    p = H(c),
    w = "." + c + ":",
    _ = "prependListener",
    R = "." + _ + ":",
    I = function (k, d, Z) {
      if (k.isRemoved) return;
      let x = k.callback;
      typeof x == "object" &&
        x.handleEvent &&
        ((k.callback = (m) => x.handleEvent(m)), (k.originalDelegate = x));
      let q;
      try {
        k.invoke(k, d, [Z]);
      } catch (m) {
        q = m;
      }
      let G = k.options;
      if (G && typeof G == "object" && G.once) {
        let m = k.originalDelegate ? k.originalDelegate : k.callback;
        d[f].call(d, Z.type, m, G);
      }
      return q;
    };
  function j(k, d, Z) {
    if (((d = d || e.event), !d)) return;
    let x = k || d.target || e,
      q = x[ne[d.type][Z ? ae : le]];
    if (q) {
      let G = [];
      if (q.length === 1) {
        let m = I(q[0], x, d);
        m && G.push(m);
      } else {
        let m = q.slice();
        for (let U = 0; U < m.length && !(d && d[ct] === !0); U++) {
          let C = I(m[U], x, d);
          C && G.push(C);
        }
      }
      if (G.length === 1) throw G[0];
      for (let m = 0; m < G.length; m++) {
        let U = G[m];
        t.nativeScheduleMicroTask(() => {
          throw U;
        });
      }
    }
  }
  let B = function (k) {
      return j(this, k, !1);
    },
    Y = function (k) {
      return j(this, k, !0);
    };
  function $(k, d) {
    if (!k) return !1;
    let Z = !0;
    d && d.useG !== void 0 && (Z = d.useG);
    let x = d && d.vh,
      q = !0;
    d && d.chkDup !== void 0 && (q = d.chkDup);
    let G = !1;
    d && d.rt !== void 0 && (G = d.rt);
    let m = k;
    for (; m && !m.hasOwnProperty(c); ) m = He(m);
    if ((!m && k[c] && (m = k), !m || m[p])) return !1;
    let U = d && d.eventNameToString,
      C = {},
      P = (m[p] = m[c]),
      b = (m[H(f)] = m[f]),
      D = (m[H(E)] = m[E]),
      K = (m[H(T)] = m[T]),
      W;
    d && d.prepend && (W = m[H(d.prepend)] = m[d.prepend]);
    function L(r, u) {
      return !ye && typeof r == "object" && r
        ? !!r.capture
        : !ye || !u
          ? r
          : typeof r == "boolean"
            ? { capture: r, passive: !0 }
            : r
              ? typeof r == "object" && r.passive !== !1
                ? { ...r, passive: !0 }
                : r
              : { passive: !0 };
    }
    let s = function (r) {
        if (!C.isExisting)
          return P.call(C.target, C.eventName, C.capture ? Y : B, C.options);
      },
      i = function (r) {
        if (!r.isRemoved) {
          let u = ne[r.eventName],
            v;
          u && (v = u[r.capture ? ae : le]);
          let S = v && r.target[v];
          if (S) {
            for (let y = 0; y < S.length; y++)
              if (S[y] === r) {
                (S.splice(y, 1),
                  (r.isRemoved = !0),
                  r.removeAbortListener &&
                    (r.removeAbortListener(), (r.removeAbortListener = null)),
                  S.length === 0 &&
                    ((r.allRemoved = !0), (r.target[v] = null)));
                break;
              }
          }
        }
        if (r.allRemoved)
          return b.call(r.target, r.eventName, r.capture ? Y : B, r.options);
      },
      o = function (r) {
        return P.call(C.target, C.eventName, r.invoke, C.options);
      },
      g = function (r) {
        return W.call(C.target, C.eventName, r.invoke, C.options);
      },
      F = function (r) {
        return b.call(r.target, r.eventName, r.invoke, r.options);
      },
      ee = Z ? s : o,
      A = Z ? i : F,
      he = function (r, u) {
        let v = typeof u;
        return (
          (v === "function" && r.callback === u) ||
          (v === "object" && r.originalDelegate === u)
        );
      },
      _e = d && d.diff ? d.diff : he,
      Q = Zone[H("UNPATCHED_EVENTS")],
      Ee = e[H("PASSIVE_EVENTS")];
    function h(r) {
      if (typeof r == "object" && r !== null) {
        let u = { ...r };
        return (r.signal && (u.signal = r.signal), u);
      }
      return r;
    }
    let l = function (r, u, v, S, y = !1, N = !1) {
      return function () {
        let O = this || e,
          M = arguments[0];
        d && d.transferEventName && (M = d.transferEventName(M));
        let V = arguments[1];
        if (!V) return r.apply(this, arguments);
        if (Ce && M === "uncaughtException") return r.apply(this, arguments);
        let z = !1;
        if (typeof V != "function") {
          if (!V.handleEvent) return r.apply(this, arguments);
          z = !0;
        }
        if (x && !x(r, V, O, arguments)) return;
        let de = ye && !!Ee && Ee.indexOf(M) !== -1,
          se = h(L(arguments[2], de)),
          Te = se?.signal;
        if (Te?.aborted) return;
        if (Q) {
          for (let ie = 0; ie < Q.length; ie++)
            if (M === Q[ie])
              return de ? r.call(O, M, V, se) : r.apply(this, arguments);
        }
        let Ne = se ? (typeof se == "boolean" ? !0 : se.capture) : !1,
          Be = se && typeof se == "object" ? se.once : !1,
          ht = Zone.current,
          Oe = ne[M];
        Oe || (at(M, U), (Oe = ne[M]));
        let Ue = Oe[Ne ? ae : le],
          ge = O[Ue],
          We = !1;
        if (ge) {
          if (((We = !0), q)) {
            for (let ie = 0; ie < ge.length; ie++) if (_e(ge[ie], V)) return;
          }
        } else ge = O[Ue] = [];
        let Pe,
          Xe = O.constructor.name,
          qe = st[Xe];
        (qe && (Pe = qe[M]),
          Pe || (Pe = Xe + u + (U ? U(M) : M)),
          (C.options = se),
          Be && (C.options.once = !1),
          (C.target = O),
          (C.capture = Ne),
          (C.eventName = M),
          (C.isExisting = We));
        let ke = Z ? bt : void 0;
        (ke && (ke.taskData = C), Te && (C.options.signal = void 0));
        let re = ht.scheduleEventTask(Pe, V, ke, v, S);
        if (Te) {
          C.options.signal = Te;
          let ie = () => re.zone.cancelTask(re);
          (r.call(Te, "abort", ie, { once: !0 }),
            (re.removeAbortListener = () =>
              Te.removeEventListener("abort", ie)));
        }
        if (
          ((C.target = null),
          ke && (ke.taskData = null),
          Be && (C.options.once = !0),
          (!ye && typeof re.options == "boolean") || (re.options = se),
          (re.target = O),
          (re.capture = Ne),
          (re.eventName = M),
          z && (re.originalDelegate = V),
          N ? ge.unshift(re) : ge.push(re),
          y)
        )
          return O;
      };
    };
    return (
      (m[c] = l(P, w, ee, A, G)),
      W && (m[_] = l(W, R, g, A, G, !0)),
      (m[f] = function () {
        let r = this || e,
          u = arguments[0];
        d && d.transferEventName && (u = d.transferEventName(u));
        let v = arguments[2],
          S = v ? (typeof v == "boolean" ? !0 : v.capture) : !1,
          y = arguments[1];
        if (!y) return b.apply(this, arguments);
        if (x && !x(b, y, r, arguments)) return;
        let N = ne[u],
          O;
        N && (O = N[S ? ae : le]);
        let M = O && r[O];
        if (M)
          for (let V = 0; V < M.length; V++) {
            let z = M[V];
            if (_e(z, y)) {
              if (
                (M.splice(V, 1),
                (z.isRemoved = !0),
                M.length === 0 &&
                  ((z.allRemoved = !0),
                  (r[O] = null),
                  !S && typeof u == "string"))
              ) {
                let de = Re + "ON_PROPERTY" + u;
                r[de] = null;
              }
              return (z.zone.cancelTask(z), G ? r : void 0);
            }
          }
        return b.apply(this, arguments);
      }),
      (m[E] = function () {
        let r = this || e,
          u = arguments[0];
        d && d.transferEventName && (u = d.transferEventName(u));
        let v = [],
          S = lt(r, U ? U(u) : u);
        for (let y = 0; y < S.length; y++) {
          let N = S[y],
            O = N.originalDelegate ? N.originalDelegate : N.callback;
          v.push(O);
        }
        return v;
      }),
      (m[T] = function () {
        let r = this || e,
          u = arguments[0];
        if (u) {
          d && d.transferEventName && (u = d.transferEventName(u));
          let v = ne[u];
          if (v) {
            let S = v[le],
              y = v[ae],
              N = r[S],
              O = r[y];
            if (N) {
              let M = N.slice();
              for (let V = 0; V < M.length; V++) {
                let z = M[V],
                  de = z.originalDelegate ? z.originalDelegate : z.callback;
                this[f].call(this, u, de, z.options);
              }
            }
            if (O) {
              let M = O.slice();
              for (let V = 0; V < M.length; V++) {
                let z = M[V],
                  de = z.originalDelegate ? z.originalDelegate : z.callback;
                this[f].call(this, u, de, z.options);
              }
            }
          }
        } else {
          let v = Object.keys(r);
          for (let S = 0; S < v.length; S++) {
            let y = v[S],
              N = it.exec(y),
              O = N && N[1];
            O && O !== "removeListener" && this[T].call(this, O);
          }
          this[T].call(this, "removeListener");
        }
        if (G) return this;
      }),
      fe(m[c], P),
      fe(m[f], b),
      K && fe(m[T], K),
      D && fe(m[E], D),
      !0
    );
  }
  let X = [];
  for (let k = 0; k < a.length; k++) X[k] = $(a[k], n);
  return X;
}
function lt(e, t) {
  if (!t) {
    let f = [];
    for (let E in e) {
      let T = it.exec(E),
        p = T && T[1];
      if (p && (!t || p === t)) {
        let w = e[E];
        if (w) for (let _ = 0; _ < w.length; _++) f.push(w[_]);
      }
    }
    return f;
  }
  let a = ne[t];
  a || (at(t), (a = ne[t]));
  let n = e[a[le]],
    c = e[a[ae]];
  return n ? (c ? n.concat(c) : n.slice()) : c ? c.slice() : [];
}
function Pt(e, t) {
  let a = e.Event;
  a &&
    a.prototype &&
    t.patchMethod(
      a.prototype,
      "stopImmediatePropagation",
      (n) =>
        function (c, f) {
          ((c[ct] = !0), n && n.apply(c, f));
        },
    );
}
function St(e, t) {
  t.patchMethod(
    e,
    "queueMicrotask",
    (a) =>
      function (n, c) {
        Zone.current.scheduleMicroTask("queueMicrotask", c[0]);
      },
  );
}
var Se = H("zoneTask");
function me(e, t, a, n) {
  let c = null,
    f = null;
  ((t += n), (a += n));
  let E = {};
  function T(w) {
    let _ = w.data;
    _.args[0] = function () {
      return w.invoke.apply(this, arguments);
    };
    let R = c.apply(e, _.args);
    return (
      Qe(R)
        ? (_.handleId = R)
        : ((_.handle = R), (_.isRefreshable = Ke(R.refresh))),
      w
    );
  }
  function p(w) {
    let { handle: _, handleId: R } = w.data;
    return f.call(e, _ ?? R);
  }
  ((c = ue(
    e,
    t,
    (w) =>
      function (_, R) {
        if (Ke(R[0])) {
          let I = {
              isRefreshable: !1,
              isPeriodic: n === "Interval",
              delay: n === "Timeout" || n === "Interval" ? R[1] || 0 : void 0,
              args: R,
            },
            j = R[0];
          R[0] = function () {
            try {
              return j.apply(this, arguments);
            } finally {
              let {
                handle: Z,
                handleId: x,
                isPeriodic: q,
                isRefreshable: G,
              } = I;
              !q && !G && (x ? delete E[x] : Z && (Z[Se] = null));
            }
          };
          let B = Ge(t, R[0], I, T, p);
          if (!B) return B;
          let {
            handleId: Y,
            handle: $,
            isRefreshable: X,
            isPeriodic: k,
          } = B.data;
          if (Y) E[Y] = B;
          else if ($ && (($[Se] = B), X && !k)) {
            let d = $.refresh;
            $.refresh = function () {
              let { zone: Z, state: x } = B;
              return (
                x === "notScheduled"
                  ? ((B._state = "scheduled"), Z._updateTaskCount(B, 1))
                  : x === "running" && (B._state = "scheduling"),
                d.call(this)
              );
            };
          }
          return $ ?? Y ?? B;
        } else return w.apply(e, R);
      },
  )),
    (f = ue(
      e,
      a,
      (w) =>
        function (_, R) {
          let I = R[0],
            j;
          (Qe(I)
            ? ((j = E[I]), delete E[I])
            : ((j = I?.[Se]), j ? (I[Se] = null) : (j = I)),
            j?.type ? j.cancelFn && j.zone.cancelTask(j) : w.apply(e, R));
        },
    )));
}
function wt(e, t) {
  let { isBrowser: a, isMix: n } = t.getGlobalObjects();
  if ((!a && !n) || !e.customElements || !("customElements" in e)) return;
  let c = [
    "connectedCallback",
    "disconnectedCallback",
    "adoptedCallback",
    "attributeChangedCallback",
    "formAssociatedCallback",
    "formDisabledCallback",
    "formResetCallback",
    "formStateRestoreCallback",
  ];
  t.patchCallbacks(t, e.customElements, "customElements", "define", c);
}
function Dt(e, t) {
  if (Zone[t.symbol("patchEventTarget")]) return;
  let {
    eventNames: a,
    zoneSymbolEventNames: n,
    TRUE_STR: c,
    FALSE_STR: f,
    ZONE_SYMBOL_PREFIX: E,
  } = t.getGlobalObjects();
  for (let p = 0; p < a.length; p++) {
    let w = a[p],
      _ = w + f,
      R = w + c,
      I = E + _,
      j = E + R;
    ((n[w] = {}), (n[w][f] = I), (n[w][c] = j));
  }
  let T = e.EventTarget;
  if (!(!T || !T.prototype))
    return (t.patchEventTarget(e, t, [T && T.prototype]), !0);
}
function Ct(e, t) {
  t.patchEventPrototype(e, t);
}
function ut(e, t, a) {
  if (!a || a.length === 0) return t;
  let n = a.filter((f) => f.target === e);
  if (!n || n.length === 0) return t;
  let c = n[0].ignoreProperties;
  return t.filter((f) => c.indexOf(f) === -1);
}
function et(e, t, a, n) {
  if (!e) return;
  let c = ut(e, t, a);
  ot(e, c, n);
}
function Ae(e) {
  return Object.getOwnPropertyNames(e)
    .filter((t) => t.startsWith("on") && t.length > 2)
    .map((t) => t.substring(2));
}
function Nt(e, t) {
  if ((Ce && !rt) || Zone[e.symbol("patchEvents")]) return;
  let a = t.__Zone_ignore_on_properties,
    n = [];
  if (ze) {
    let c = window;
    n = n.concat([
      "Document",
      "SVGElement",
      "Element",
      "HTMLElement",
      "HTMLBodyElement",
      "HTMLMediaElement",
      "HTMLFrameSetElement",
      "HTMLFrameElement",
      "HTMLIFrameElement",
      "HTMLMarqueeElement",
      "Worker",
    ]);
    let f = kt() ? [{ target: c, ignoreProperties: ["error"] }] : [];
    et(c, Ae(c), a && a.concat(f), He(c));
  }
  n = n.concat([
    "XMLHttpRequest",
    "XMLHttpRequestEventTarget",
    "IDBIndex",
    "IDBRequest",
    "IDBOpenDBRequest",
    "IDBDatabase",
    "IDBTransaction",
    "IDBCursor",
    "WebSocket",
  ]);
  for (let c = 0; c < n.length; c++) {
    let f = t[n[c]];
    f && f.prototype && et(f.prototype, Ae(f.prototype), a);
  }
}
function Ot(e) {
  (e.__load_patch("legacy", (t) => {
    let a = t[e.__symbol__("legacyPatch")];
    a && a();
  }),
    e.__load_patch("timers", (t) => {
      let a = "set",
        n = "clear";
      (me(t, a, n, "Timeout"),
        me(t, a, n, "Interval"),
        me(t, a, n, "Immediate"));
    }),
    e.__load_patch("requestAnimationFrame", (t) => {
      (me(t, "request", "cancel", "AnimationFrame"),
        me(t, "mozRequest", "mozCancel", "AnimationFrame"),
        me(t, "webkitRequest", "webkitCancel", "AnimationFrame"));
    }),
    e.__load_patch("blocking", (t, a) => {
      let n = ["alert", "prompt", "confirm"];
      for (let c = 0; c < n.length; c++) {
        let f = n[c];
        ue(
          t,
          f,
          (E, T, p) =>
            function (w, _) {
              return a.current.run(E, t, _, p);
            },
        );
      }
    }),
    e.__load_patch("EventTarget", (t, a, n) => {
      (Ct(t, n), Dt(t, n));
      let c = t.XMLHttpRequestEventTarget;
      c && c.prototype && n.patchEventTarget(t, n, [c.prototype]);
    }),
    e.__load_patch("MutationObserver", (t, a, n) => {
      (ve("MutationObserver"), ve("WebKitMutationObserver"));
    }),
    e.__load_patch("IntersectionObserver", (t, a, n) => {
      ve("IntersectionObserver");
    }),
    e.__load_patch("FileReader", (t, a, n) => {
      ve("FileReader");
    }),
    e.__load_patch("on_property", (t, a, n) => {
      Nt(n, t);
    }),
    e.__load_patch("customElements", (t, a, n) => {
      wt(t, n);
    }),
    e.__load_patch("XHR", (t, a) => {
      w(t);
      let n = H("xhrTask"),
        c = H("xhrSync"),
        f = H("xhrListener"),
        E = H("xhrScheduled"),
        T = H("xhrURL"),
        p = H("xhrErrorBeforeScheduled");
      function w(_) {
        let R = _.XMLHttpRequest;
        if (!R) return;
        let I = R.prototype;
        function j(P) {
          return P[n];
        }
        let B = I[Me],
          Y = I[Ie];
        if (!B) {
          let P = _.XMLHttpRequestEventTarget;
          if (P) {
            let b = P.prototype;
            ((B = b[Me]), (Y = b[Ie]));
          }
        }
        let $ = "readystatechange",
          X = "scheduled";
        function k(P) {
          let b = P.data,
            D = b.target;
          ((D[E] = !1), (D[p] = !1));
          let K = D[f];
          (B || ((B = D[Me]), (Y = D[Ie])), K && Y.call(D, $, K));
          let W = (D[f] = () => {
            if (D.readyState === D.DONE)
              if (!b.aborted && D[E] && P.state === X) {
                let s = D[a.__symbol__("loadfalse")];
                if (D.status !== 0 && s && s.length > 0) {
                  let i = P.invoke;
                  ((P.invoke = function () {
                    let o = D[a.__symbol__("loadfalse")];
                    for (let g = 0; g < o.length; g++)
                      o[g] === P && o.splice(g, 1);
                    !b.aborted && P.state === X && i.call(P);
                  }),
                    s.push(P));
                } else P.invoke();
              } else !b.aborted && D[E] === !1 && (D[p] = !0);
          });
          return (
            B.call(D, $, W),
            D[n] || (D[n] = P),
            U.apply(D, b.args),
            (D[E] = !0),
            P
          );
        }
        function d() {}
        function Z(P) {
          let b = P.data;
          return ((b.aborted = !0), C.apply(b.target, b.args));
        }
        let x = ue(
            I,
            "open",
            () =>
              function (P, b) {
                return ((P[c] = b[2] == !1), (P[T] = b[1]), x.apply(P, b));
              },
          ),
          q = "XMLHttpRequest.send",
          G = H("fetchTaskAborting"),
          m = H("fetchTaskScheduling"),
          U = ue(
            I,
            "send",
            () =>
              function (P, b) {
                if (a.current[m] === !0 || P[c]) return U.apply(P, b);
                {
                  let D = {
                      target: P,
                      url: P[T],
                      isPeriodic: !1,
                      args: b,
                      aborted: !1,
                    },
                    K = Ge(q, d, D, k, Z);
                  P && P[p] === !0 && !D.aborted && K.state === X && K.invoke();
                }
              },
          ),
          C = ue(
            I,
            "abort",
            () =>
              function (P, b) {
                let D = j(P);
                if (D && typeof D.type == "string") {
                  if (D.cancelFn == null || (D.data && D.data.aborted)) return;
                  D.zone.cancelTask(D);
                } else if (a.current[G] === !0) return C.apply(P, b);
              },
          );
      }
    }),
    e.__load_patch("geolocation", (t) => {
      t.navigator &&
        t.navigator.geolocation &&
        mt(t.navigator.geolocation, ["getCurrentPosition", "watchPosition"]);
    }),
    e.__load_patch("PromiseRejectionEvent", (t, a) => {
      function n(c) {
        return function (f) {
          lt(t, c).forEach((T) => {
            let p = t.PromiseRejectionEvent;
            if (p) {
              let w = new p(c, { promise: f.promise, reason: f.rejection });
              T.invoke(w);
            }
          });
        };
      }
      t.PromiseRejectionEvent &&
        ((a[H("unhandledPromiseRejectionHandler")] = n("unhandledrejection")),
        (a[H("rejectionHandledHandler")] = n("rejectionhandled")));
    }),
    e.__load_patch("queueMicrotask", (t, a, n) => {
      St(t, n);
    }));
}
function Mt(e) {
  e.__load_patch("ZoneAwarePromise", (t, a, n) => {
    let c = Object.getOwnPropertyDescriptor,
      f = Object.defineProperty;
    function E(h) {
      if (h && h.toString === Object.prototype.toString) {
        let l = h.constructor && h.constructor.name;
        return (l || "") + ": " + JSON.stringify(h);
      }
      return h ? h.toString() : Object.prototype.toString.call(h);
    }
    let T = n.symbol,
      p = [],
      w = t[T("DISABLE_WRAPPING_UNCAUGHT_PROMISE_REJECTION")] !== !1,
      _ = T("Promise"),
      R = T("then"),
      I = "__creationTrace__";
    ((n.onUnhandledError = (h) => {
      if (n.showUncaughtError()) {
        let l = h && h.rejection;
        l
          ? console.error(
              "Unhandled Promise rejection:",
              l instanceof Error ? l.message : l,
              "; Zone:",
              h.zone.name,
              "; Task:",
              h.task && h.task.source,
              "; Value:",
              l,
              l instanceof Error ? l.stack : void 0,
            )
          : console.error(h);
      }
    }),
      (n.microtaskDrainDone = () => {
        for (; p.length; ) {
          let h = p.shift();
          try {
            h.zone.runGuarded(() => {
              throw h.throwOriginal ? h.rejection : h;
            });
          } catch (l) {
            B(l);
          }
        }
      }));
    let j = T("unhandledPromiseRejectionHandler");
    function B(h) {
      n.onUnhandledError(h);
      try {
        let l = a[j];
        typeof l == "function" && l.call(this, h);
      } catch {}
    }
    function Y(h) {
      return h && h.then;
    }
    function $(h) {
      return h;
    }
    function X(h) {
      return A.reject(h);
    }
    let k = T("state"),
      d = T("value"),
      Z = T("finally"),
      x = T("parentPromiseValue"),
      q = T("parentPromiseState"),
      G = "Promise.then",
      m = null,
      U = !0,
      C = !1,
      P = 0;
    function b(h, l) {
      return (r) => {
        try {
          L(h, l, r);
        } catch (u) {
          L(h, !1, u);
        }
      };
    }
    let D = function () {
        let h = !1;
        return function (r) {
          return function () {
            h || ((h = !0), r.apply(null, arguments));
          };
        };
      },
      K = "Promise resolved with itself",
      W = T("currentTaskTrace");
    function L(h, l, r) {
      let u = D();
      if (h === r) throw new TypeError(K);
      if (h[k] === m) {
        let v = null;
        try {
          (typeof r == "object" || typeof r == "function") && (v = r && r.then);
        } catch (S) {
          return (
            u(() => {
              L(h, !1, S);
            })(),
            h
          );
        }
        if (
          l !== C &&
          r instanceof A &&
          r.hasOwnProperty(k) &&
          r.hasOwnProperty(d) &&
          r[k] !== m
        )
          (i(r), L(h, r[k], r[d]));
        else if (l !== C && typeof v == "function")
          try {
            v.call(r, u(b(h, l)), u(b(h, !1)));
          } catch (S) {
            u(() => {
              L(h, !1, S);
            })();
          }
        else {
          h[k] = l;
          let S = h[d];
          if (
            ((h[d] = r),
            h[Z] === Z && l === U && ((h[k] = h[q]), (h[d] = h[x])),
            l === C && r instanceof Error)
          ) {
            let y =
              a.currentTask && a.currentTask.data && a.currentTask.data[I];
            y &&
              f(r, W, {
                configurable: !0,
                enumerable: !1,
                writable: !0,
                value: y,
              });
          }
          for (let y = 0; y < S.length; ) o(h, S[y++], S[y++], S[y++], S[y++]);
          if (S.length == 0 && l == C) {
            h[k] = P;
            let y = r;
            try {
              throw new Error(
                "Uncaught (in promise): " +
                  E(r) +
                  (r && r.stack
                    ? `
` + r.stack
                    : ""),
              );
            } catch (N) {
              y = N;
            }
            (w && (y.throwOriginal = !0),
              (y.rejection = r),
              (y.promise = h),
              (y.zone = a.current),
              (y.task = a.currentTask),
              p.push(y),
              n.scheduleMicroTask());
          }
        }
      }
      return h;
    }
    let s = T("rejectionHandledHandler");
    function i(h) {
      if (h[k] === P) {
        try {
          let l = a[s];
          l &&
            typeof l == "function" &&
            l.call(this, { rejection: h[d], promise: h });
        } catch {}
        h[k] = C;
        for (let l = 0; l < p.length; l++) h === p[l].promise && p.splice(l, 1);
      }
    }
    function o(h, l, r, u, v) {
      i(h);
      let S = h[k],
        y = S
          ? typeof u == "function"
            ? u
            : $
          : typeof v == "function"
            ? v
            : X;
      l.scheduleMicroTask(
        G,
        () => {
          try {
            let N = h[d],
              O = !!r && Z === r[Z];
            O && ((r[x] = N), (r[q] = S));
            let M = l.run(y, void 0, O && y !== X && y !== $ ? [] : [N]);
            L(r, !0, M);
          } catch (N) {
            L(r, !1, N);
          }
        },
        r,
      );
    }
    let g = "function ZoneAwarePromise() { [native code] }",
      F = function () {},
      ee = t.AggregateError;
    class A {
      static toString() {
        return g;
      }
      static resolve(l) {
        return l instanceof A ? l : L(new this(null), U, l);
      }
      static reject(l) {
        return L(new this(null), C, l);
      }
      static withResolvers() {
        let l = {};
        return (
          (l.promise = new A((r, u) => {
            ((l.resolve = r), (l.reject = u));
          })),
          l
        );
      }
      static any(l) {
        if (!l || typeof l[Symbol.iterator] != "function")
          return Promise.reject(new ee([], "All promises were rejected"));
        let r = [],
          u = 0;
        try {
          for (let y of l) (u++, r.push(A.resolve(y)));
        } catch {
          return Promise.reject(new ee([], "All promises were rejected"));
        }
        if (u === 0)
          return Promise.reject(new ee([], "All promises were rejected"));
        let v = !1,
          S = [];
        return new A((y, N) => {
          for (let O = 0; O < r.length; O++)
            r[O].then(
              (M) => {
                v || ((v = !0), y(M));
              },
              (M) => {
                (S.push(M),
                  u--,
                  u === 0 &&
                    ((v = !0), N(new ee(S, "All promises were rejected"))));
              },
            );
        });
      }
      static race(l) {
        let r,
          u,
          v = new this((N, O) => {
            ((r = N), (u = O));
          });
        function S(N) {
          r(N);
        }
        function y(N) {
          u(N);
        }
        for (let N of l) (Y(N) || (N = this.resolve(N)), N.then(S, y));
        return v;
      }
      static all(l) {
        return A.allWithCallback(l);
      }
      static allSettled(l) {
        return (this && this.prototype instanceof A ? this : A).allWithCallback(
          l,
          {
            thenCallback: (u) => ({ status: "fulfilled", value: u }),
            errorCallback: (u) => ({ status: "rejected", reason: u }),
          },
        );
      }
      static allWithCallback(l, r) {
        let u,
          v,
          S = new this((M, V) => {
            ((u = M), (v = V));
          }),
          y = 2,
          N = 0,
          O = [];
        for (let M of l) {
          Y(M) || (M = this.resolve(M));
          let V = N;
          try {
            M.then(
              (z) => {
                ((O[V] = r ? r.thenCallback(z) : z), y--, y === 0 && u(O));
              },
              (z) => {
                r ? ((O[V] = r.errorCallback(z)), y--, y === 0 && u(O)) : v(z);
              },
            );
          } catch (z) {
            v(z);
          }
          (y++, N++);
        }
        return ((y -= 2), y === 0 && u(O), S);
      }
      constructor(l) {
        let r = this;
        if (!(r instanceof A))
          throw new Error("Must be an instanceof Promise.");
        ((r[k] = m), (r[d] = []));
        try {
          let u = D();
          l && l(u(b(r, U)), u(b(r, C)));
        } catch (u) {
          L(r, !1, u);
        }
      }
      get [Symbol.toStringTag]() {
        return "Promise";
      }
      get [Symbol.species]() {
        return A;
      }
      then(l, r) {
        let u = this.constructor?.[Symbol.species];
        (!u || typeof u != "function") && (u = this.constructor || A);
        let v = new u(F),
          S = a.current;
        return (
          this[k] == m ? this[d].push(S, v, l, r) : o(this, S, v, l, r),
          v
        );
      }
      catch(l) {
        return this.then(null, l);
      }
      finally(l) {
        let r = this.constructor?.[Symbol.species];
        (!r || typeof r != "function") && (r = A);
        let u = new r(F);
        u[Z] = Z;
        let v = a.current;
        return (
          this[k] == m ? this[d].push(v, u, l, l) : o(this, v, u, l, l),
          u
        );
      }
    }
    ((A.resolve = A.resolve),
      (A.reject = A.reject),
      (A.race = A.race),
      (A.all = A.all));
    let he = (t[_] = t.Promise);
    t.Promise = A;
    let _e = T("thenPatched");
    function Q(h) {
      let l = h.prototype,
        r = c(l, "then");
      if (r && (r.writable === !1 || !r.configurable)) return;
      let u = l.then;
      ((l[R] = u),
        (h.prototype.then = function (v, S) {
          return new A((N, O) => {
            u.call(this, N, O);
          }).then(v, S);
        }),
        (h[_e] = !0));
    }
    n.patchThen = Q;
    function Ee(h) {
      return function (l, r) {
        let u = h.apply(l, r);
        if (u instanceof A) return u;
        let v = u.constructor;
        return (v[_e] || Q(v), u);
      };
    }
    return (
      he && (Q(he), ue(t, "fetch", (h) => Ee(h))),
      (Promise[a.__symbol__("uncaughtPromiseErrors")] = p),
      A
    );
  });
}
function It(e) {
  e.__load_patch("toString", (t) => {
    let a = Function.prototype.toString,
      n = H("OriginalDelegate"),
      c = H("Promise"),
      f = H("Error"),
      E = function () {
        if (typeof this == "function") {
          let _ = this[n];
          if (_)
            return typeof _ == "function"
              ? a.call(_)
              : Object.prototype.toString.call(_);
          if (this === Promise) {
            let R = t[c];
            if (R) return a.call(R);
          }
          if (this === Error) {
            let R = t[f];
            if (R) return a.call(R);
          }
        }
        return a.call(this);
      };
    ((E[n] = a), (Function.prototype.toString = E));
    let T = Object.prototype.toString,
      p = "[object Promise]";
    Object.prototype.toString = function () {
      return typeof Promise == "function" && this instanceof Promise
        ? p
        : T.call(this);
    };
  });
}
function Lt(e, t, a, n, c) {
  let f = Zone.__symbol__(n);
  if (t[f]) return;
  let E = (t[f] = t[n]);
  ((t[n] = function (T, p, w) {
    return (
      p &&
        p.prototype &&
        c.forEach(function (_) {
          let R = `${a}.${n}::` + _,
            I = p.prototype;
          try {
            if (I.hasOwnProperty(_)) {
              let j = e.ObjectGetOwnPropertyDescriptor(I, _);
              j && j.value
                ? ((j.value = e.wrapWithCurrentZone(j.value, R)),
                  e._redefineProperty(p.prototype, _, j))
                : I[_] && (I[_] = e.wrapWithCurrentZone(I[_], R));
            } else I[_] && (I[_] = e.wrapWithCurrentZone(I[_], R));
          } catch {}
        }),
      E.call(t, T, p, w)
    );
  }),
    e.attachOriginToPatched(t[n], E));
}
function At(e) {
  e.__load_patch("util", (t, a, n) => {
    let c = Ae(t);
    ((n.patchOnProperties = ot),
      (n.patchMethod = ue),
      (n.bindArguments = Ve),
      (n.patchMacroTask = pt));
    let f = a.__symbol__("BLACK_LISTED_EVENTS"),
      E = a.__symbol__("UNPATCHED_EVENTS");
    (t[E] && (t[f] = t[E]),
      t[f] && (a[f] = a[E] = t[f]),
      (n.patchEventPrototype = Pt),
      (n.patchEventTarget = Rt),
      (n.isIEOrEdge = vt),
      (n.ObjectDefineProperty = Ze),
      (n.ObjectGetOwnPropertyDescriptor = be),
      (n.ObjectCreate = Et),
      (n.ArraySlice = Tt),
      (n.patchClass = ve),
      (n.wrapWithCurrentZone = Fe),
      (n.filterProperties = ut),
      (n.attachOriginToPatched = fe),
      (n._redefineProperty = Object.defineProperty),
      (n.patchCallbacks = Lt),
      (n.getGlobalObjects = () => ({
        globalSources: st,
        zoneSymbolEventNames: ne,
        eventNames: c,
        isBrowser: ze,
        isMix: rt,
        isNode: Ce,
        TRUE_STR: ae,
        FALSE_STR: le,
        ZONE_SYMBOL_PREFIX: Re,
        ADD_EVENT_LISTENER_STR: je,
        REMOVE_EVENT_LISTENER_STR: xe,
      })));
  });
}
function Zt(e) {
  (Mt(e), It(e), At(e));
}
var ft = _t();
Zt(ft);
Ot(ft);
(globalThis.$localize ??= {}).locale =
  "en-US"; /**i18n:e2f94bf06bdfc8c8ab493a12299261c375fc525ae09e041ca331cb13279050ab*/
