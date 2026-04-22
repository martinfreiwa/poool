import { v as mo } from "./chunk-NG4JWF4P.js";
import {
  b as ma,
  c as _a,
  d as We,
  e as _o,
  f as ha,
  g as Fa,
  h as si,
  k as Sa,
  l as Pa,
  m as Oa,
} from "./chunk-O3LQP76T.js";
import { a as go, b as fa, c as va } from "./chunk-MO34KLTL.js";
import {
  $a as _e,
  $b as ie,
  Ba as M,
  Cb as ia,
  Da as ao,
  Db as ai,
  Eb as bn,
  F as De,
  Ha as tn,
  Ia as yn,
  Ib as oa,
  Ja as na,
  K as se,
  Ka as vt,
  L as ce,
  La as yt,
  M as Zr,
  Ma as l,
  N as ea,
  Na as p,
  Oa as f,
  Pa as ni,
  Q as ta,
  Qa as ii,
  S as io,
  Sa as Ht,
  Sb as li,
  Ua as V,
  Va as X,
  Vb as co,
  Wa as Y,
  Wb as ra,
  X as Hn,
  Xa as qo,
  Ya as Jo,
  Za as Et,
  Zb as aa,
  _ as oo,
  _a as pe,
  _b as po,
  ac as uo,
  ca as vn,
  cb as lo,
  da as Ot,
  db as gt,
  dc as la,
  ea as ro,
  eb as mt,
  ec as sa,
  fb as _t,
  g as jr,
  ga as F,
  hc as ca,
  j as ti,
  jc as pa,
  kb as y,
  kc as da,
  l as Yt,
  lb as Rt,
  lc as ua,
  m as Kr,
  mb as ne,
  mc as ga,
  n as Yo,
  nc as xn,
  o as Yr,
  pb as oi,
  q as Uo,
  qb as ri,
  ra as J,
  rb as En,
  sb as Qo,
  t as Ur,
  tb as Zo,
  u as qr,
  ub as er,
  v as Jr,
  vb as bt,
  wb as zn,
  xa as Nt,
  y as Qr,
  yb as tr,
  zb as so,
} from "./chunk-A4X4NSFE.js";
import { d as ol, f as Xr } from "./chunk-ZBVOF6Q3.js";
var nr = ol((fo, Ea) => {
  "use strict";
  (function (c, E) {
    typeof fo == "object" && typeof Ea < "u"
      ? E(fo)
      : typeof define == "function" && define.amd
        ? define(["exports"], E)
        : ((c = c || self), E((c.window = c.window || {})));
  })(fo, function (c) {
    "use strict";
    function E(d, e) {
      for (var o = 0; o < e.length; o++) {
        var t = e[o];
        ((t.enumerable = t.enumerable || !1),
          (t.configurable = !0),
          "value" in t && (t.writable = !0),
          Object.defineProperty(d, t.key, t));
      }
    }
    function i(d, e, o) {
      return (e && E(d.prototype, e), o && E(d, o), d);
    }
    var a,
      r,
      S,
      x,
      B,
      te,
      Pe,
      et,
      de,
      xt,
      ue,
      nn,
      zt,
      or,
      rr = function () {
        return (
          a ||
          (typeof window < "u" && (a = window.gsap) && a.registerPlugin && a)
        );
      },
      ar = 1,
      Vn = [],
      H = [],
      Ut = [],
      ci = Date.now,
      vo = function (e, o) {
        return o;
      },
      za = function () {
        var e = xt.core,
          o = e.bridge || {},
          t = e._scrollers,
          n = e._proxies;
        (t.push.apply(t, H),
          n.push.apply(n, Ut),
          (H = t),
          (Ut = n),
          (vo = function (_, g) {
            return o[_](g);
          }));
      },
      gn = function (e, o) {
        return ~Ut.indexOf(e) && Ut[Ut.indexOf(e) + 1][o];
      },
      pi = function (e) {
        return !!~ue.indexOf(e);
      },
      tt = function (e, o, t, n, s) {
        return e.addEventListener(o, t, { passive: n !== !1, capture: !!s });
      },
      nt = function (e, o, t, n) {
        return e.removeEventListener(o, t, !!n);
      },
      Di = "scrollLeft",
      Mi = "scrollTop",
      yo = function () {
        return (nn && nn.isPressed) || H.cache++;
      },
      Ni = function (e, o) {
        var t = function n(s) {
          if (s || s === 0) {
            ar && (x.history.scrollRestoration = "manual");
            var _ = nn && nn.isPressed;
            ((s = n.v = Math.round(s) || (nn && nn.iOS ? 1 : 0)),
              e(s),
              (n.cacheID = H.cache),
              _ && vo("ss", s));
          } else
            (o || H.cache !== n.cacheID || vo("ref")) &&
              ((n.cacheID = H.cache), (n.v = e()));
          return n.v + n.offset;
        };
        return ((t.offset = 0), e && t);
      },
      it = {
        s: Di,
        p: "left",
        p2: "Left",
        os: "right",
        os2: "Right",
        d: "width",
        d2: "Width",
        a: "x",
        sc: Ni(function (d) {
          return arguments.length
            ? x.scrollTo(d, Me.sc())
            : x.pageXOffset || B[Di] || te[Di] || Pe[Di] || 0;
        }),
      },
      Me = {
        s: Mi,
        p: "top",
        p2: "Top",
        os: "bottom",
        os2: "Bottom",
        d: "height",
        d2: "Height",
        a: "y",
        op: it,
        sc: Ni(function (d) {
          return arguments.length
            ? x.scrollTo(it.sc(), d)
            : x.pageYOffset || B[Mi] || te[Mi] || Pe[Mi] || 0;
        }),
      },
      ht = function (e, o) {
        return (
          ((o && o._ctx && o._ctx.selector) || a.utils.toArray)(e)[0] ||
          (typeof e == "string" && a.config().nullTargetWarn !== !1
            ? console.warn("Element not found:", e)
            : null)
        );
      },
      mn = function (e, o) {
        var t = o.s,
          n = o.sc;
        pi(e) && (e = B.scrollingElement || te);
        var s = H.indexOf(e),
          _ = n === Me.sc ? 1 : 2;
        (!~s && (s = H.push(e) - 1), H[s + _] || tt(e, "scroll", yo));
        var g = H[s + _],
          O =
            g ||
            (H[s + _] =
              Ni(gn(e, t), !0) ||
              (pi(e)
                ? n
                : Ni(function (D) {
                    return arguments.length ? (e[t] = D) : e[t];
                  })));
        return (
          (O.target = e),
          g || (O.smooth = a.getProperty(e, "scrollBehavior") === "smooth"),
          O
        );
      },
      Eo = function (e, o, t) {
        var n = e,
          s = e,
          _ = ci(),
          g = _,
          O = o || 50,
          D = Math.max(500, O * 3),
          Q = function (T, Fe) {
            var le = ci();
            Fe || le - _ > O
              ? ((s = n), (n = T), (g = _), (_ = le))
              : t
                ? (n += T)
                : (n = s + ((T - s) / (le - g)) * (_ - g));
          },
          j = function () {
            ((s = n = t ? 0 : n), (g = _ = 0));
          },
          C = function (T) {
            var Fe = g,
              le = s,
              Re = ci();
            return (
              (T || T === 0) && T !== n && Q(T),
              _ === g || Re - g > D
                ? 0
                : ((n + (t ? le : -le)) / ((t ? Re : _) - Fe)) * 1e3
            );
          };
        return { update: Q, reset: j, getVelocity: C };
      },
      di = function (e, o) {
        return (
          o && !e._gsapAllow && e.preventDefault(),
          e.changedTouches ? e.changedTouches[0] : e
        );
      },
      lr = function (e) {
        var o = Math.max.apply(Math, e),
          t = Math.min.apply(Math, e);
        return Math.abs(o) >= Math.abs(t) ? o : t;
      },
      sr = function () {
        ((xt = a.core.globals().ScrollTrigger), xt && xt.core && za());
      },
      cr = function (e) {
        return (
          (a = e || rr()),
          !r &&
            a &&
            typeof document < "u" &&
            document.body &&
            ((x = window),
            (B = document),
            (te = B.documentElement),
            (Pe = B.body),
            (ue = [x, B, te, Pe]),
            (S = a.utils.clamp),
            (or = a.core.context || function () {}),
            (de = "onpointerenter" in Pe ? "pointer" : "mouse"),
            (et = Oe.isTouch =
              x.matchMedia &&
              x.matchMedia("(hover: none), (pointer: coarse)").matches
                ? 1
                : "ontouchstart" in x ||
                    navigator.maxTouchPoints > 0 ||
                    navigator.msMaxTouchPoints > 0
                  ? 2
                  : 0),
            (zt = Oe.eventTypes =
              (
                "ontouchstart" in te
                  ? "touchstart,touchmove,touchcancel,touchend"
                  : "onpointerdown" in te
                    ? "pointerdown,pointermove,pointercancel,pointerup"
                    : "mousedown,mousemove,mouseup,mouseup"
              ).split(",")),
            setTimeout(function () {
              return (ar = 0);
            }, 500),
            sr(),
            (r = 1)),
          r
        );
      };
    ((it.op = Me), (H.cache = 0));
    var Oe = (function () {
      function d(o) {
        this.init(o);
      }
      var e = d.prototype;
      return (
        (e.init = function (t) {
          (r || cr(a) || console.warn("Please gsap.registerPlugin(Observer)"),
            xt || sr());
          var n = t.tolerance,
            s = t.dragMinimum,
            _ = t.type,
            g = t.target,
            O = t.lineHeight,
            D = t.debounce,
            Q = t.preventDefault,
            j = t.onStop,
            C = t.onStopDelay,
            h = t.ignore,
            T = t.wheelSpeed,
            Fe = t.event,
            le = t.onDragStart,
            Re = t.onDragEnd,
            fe = t.onDrag,
            Ve = t.onPress,
            $ = t.onRelease,
            $t = t.onRight,
            ge = t.onLeft,
            R = t.onUp,
            lt = t.onDown,
            Ft = t.onChangeX,
            b = t.onChangeY,
            Be = t.onChange,
            N = t.onToggleX,
            an = t.onToggleY,
            Le = t.onHover,
            st = t.onHoverEnd,
            ct = t.onMove,
            oe = t.ignoreCheck,
            Ce = t.isNormalizer,
            Ae = t.onGestureStart,
            u = t.onGestureEnd,
            ke = t.onWheel,
            Ln = t.onEnable,
            fn = t.onDisable,
            Gt = t.onClick,
            ln = t.scrollSpeed,
            pt = t.capture,
            Te = t.allowClicks,
            dt = t.lockAxis,
            qe = t.onLockAxis;
          ((this.target = g = ht(g) || te),
            (this.vars = t),
            h && (h = a.utils.toArray(h)),
            (n = n || 1e-9),
            (s = s || 0),
            (T = T || 1),
            (ln = ln || 1),
            (_ = _ || "wheel,touch,pointer"),
            (D = D !== !1),
            O || (O = parseFloat(x.getComputedStyle(Pe).lineHeight) || 22));
          var hn,
            ut,
            St,
            q,
            ve,
            Pt,
            wt,
            m = this,
            Dt = 0,
            sn = 0,
            Fn = t.passive || (!Q && t.passive !== !1),
            ye = mn(g, it),
            cn = mn(g, Me),
            Sn = ye(),
            kn = cn(),
            He =
              ~_.indexOf("touch") &&
              !~_.indexOf("pointer") &&
              zt[0] === "pointerdown",
            Pn = pi(g),
            Ee = g.ownerDocument || B,
            Wt = [0, 0, 0],
            Bt = [0, 0, 0],
            pn = 0,
            xi = function () {
              return (pn = ci());
            },
            we = function (w, Z) {
              return (
                ((m.event = w) && h && ~h.indexOf(w.target)) ||
                (Z && He && w.pointerType !== "touch") ||
                (oe && oe(w, Z))
              );
            },
            eo = function () {
              (m._vx.reset(), m._vy.reset(), ut.pause(), j && j(m));
            },
            dn = function () {
              var w = (m.deltaX = lr(Wt)),
                Z = (m.deltaY = lr(Bt)),
                P = Math.abs(w) >= n,
                L = Math.abs(Z) >= n;
              (Be && (P || L) && Be(m, w, Z, Wt, Bt),
                P &&
                  ($t && m.deltaX > 0 && $t(m),
                  ge && m.deltaX < 0 && ge(m),
                  Ft && Ft(m),
                  N && m.deltaX < 0 != Dt < 0 && N(m),
                  (Dt = m.deltaX),
                  (Wt[0] = Wt[1] = Wt[2] = 0)),
                L &&
                  (lt && m.deltaY > 0 && lt(m),
                  R && m.deltaY < 0 && R(m),
                  b && b(m),
                  an && m.deltaY < 0 != sn < 0 && an(m),
                  (sn = m.deltaY),
                  (Bt[0] = Bt[1] = Bt[2] = 0)),
                (q || St) &&
                  (ct && ct(m),
                  St && (le && St === 1 && le(m), fe && fe(m), (St = 0)),
                  (q = !1)),
                Pt && !(Pt = !1) && qe && qe(m),
                ve && (ke(m), (ve = !1)),
                (hn = 0));
            },
            Qn = function (w, Z, P) {
              ((Wt[P] += w),
                (Bt[P] += Z),
                m._vx.update(w),
                m._vy.update(Z),
                D ? hn || (hn = requestAnimationFrame(dn)) : dn());
            },
            Zn = function (w, Z) {
              (dt &&
                !wt &&
                ((m.axis = wt = Math.abs(w) > Math.abs(Z) ? "x" : "y"),
                (Pt = !0)),
                wt !== "y" && ((Wt[2] += w), m._vx.update(w, !0)),
                wt !== "x" && ((Bt[2] += Z), m._vy.update(Z, !0)),
                D ? hn || (hn = requestAnimationFrame(dn)) : dn());
            },
            On = function (w) {
              if (!we(w, 1)) {
                w = di(w, Q);
                var Z = w.clientX,
                  P = w.clientY,
                  L = Z - m.x,
                  A = P - m.y,
                  k = m.isDragging;
                ((m.x = Z),
                  (m.y = P),
                  (k ||
                    ((L || A) &&
                      (Math.abs(m.startX - Z) >= s ||
                        Math.abs(m.startY - P) >= s))) &&
                    ((St = k ? 2 : 1), k || (m.isDragging = !0), Zn(L, A)));
              }
            },
            In = (m.onPress = function (I) {
              we(I, 1) ||
                (I && I.button) ||
                ((m.axis = wt = null),
                ut.pause(),
                (m.isPressed = !0),
                (I = di(I)),
                (Dt = sn = 0),
                (m.startX = m.x = I.clientX),
                (m.startY = m.y = I.clientY),
                m._vx.reset(),
                m._vy.reset(),
                tt(Ce ? g : Ee, zt[1], On, Fn, !0),
                (m.deltaX = m.deltaY = 0),
                Ve && Ve(m));
            }),
            K = (m.onRelease = function (I) {
              if (!we(I, 1)) {
                nt(Ce ? g : Ee, zt[1], On, !0);
                var w = !isNaN(m.y - m.startY),
                  Z = m.isDragging,
                  P =
                    Z &&
                    (Math.abs(m.x - m.startX) > 3 ||
                      Math.abs(m.y - m.startY) > 3),
                  L = di(I);
                (!P &&
                  w &&
                  (m._vx.reset(),
                  m._vy.reset(),
                  Q &&
                    Te &&
                    a.delayedCall(0.08, function () {
                      if (ci() - pn > 300 && !I.defaultPrevented) {
                        if (I.target.click) I.target.click();
                        else if (Ee.createEvent) {
                          var A = Ee.createEvent("MouseEvents");
                          (A.initMouseEvent(
                            "click",
                            !0,
                            !0,
                            x,
                            1,
                            L.screenX,
                            L.screenY,
                            L.clientX,
                            L.clientY,
                            !1,
                            !1,
                            !1,
                            !1,
                            0,
                            null,
                          ),
                            I.target.dispatchEvent(A));
                        }
                      }
                    })),
                  (m.isDragging = m.isGesturing = m.isPressed = !1),
                  j && Z && !Ce && ut.restart(!0),
                  St && dn(),
                  Re && Z && Re(m),
                  $ && $(m, P));
              }
            }),
            $n = function (w) {
              return (
                w.touches &&
                w.touches.length > 1 &&
                (m.isGesturing = !0) &&
                Ae(w, m.isDragging)
              );
            },
            Vt = function () {
              return (m.isGesturing = !1) || u(m);
            },
            Xt = function (w) {
              if (!we(w)) {
                var Z = ye(),
                  P = cn();
                (Qn((Z - Sn) * ln, (P - kn) * ln, 1),
                  (Sn = Z),
                  (kn = P),
                  j && ut.restart(!0));
              }
            },
            jt = function (w) {
              if (!we(w)) {
                ((w = di(w, Q)), ke && (ve = !0));
                var Z =
                  (w.deltaMode === 1
                    ? O
                    : w.deltaMode === 2
                      ? x.innerHeight
                      : 1) * T;
                (Qn(w.deltaX * Z, w.deltaY * Z, 0), j && !Ce && ut.restart(!0));
              }
            },
            Gn = function (w) {
              if (!we(w)) {
                var Z = w.clientX,
                  P = w.clientY,
                  L = Z - m.x,
                  A = P - m.y;
                ((m.x = Z),
                  (m.y = P),
                  (q = !0),
                  j && ut.restart(!0),
                  (L || A) && Zn(L, A));
              }
            },
            ei = function (w) {
              ((m.event = w), Le(m));
            },
            un = function (w) {
              ((m.event = w), st(m));
            },
            Ci = function (w) {
              return we(w) || (di(w, Q) && Gt(m));
            };
          ((ut = m._dc = a.delayedCall(C || 0.25, eo).pause()),
            (m.deltaX = m.deltaY = 0),
            (m._vx = Eo(0, 50, !0)),
            (m._vy = Eo(0, 50, !0)),
            (m.scrollX = ye),
            (m.scrollY = cn),
            (m.isDragging = m.isGesturing = m.isPressed = !1),
            or(this),
            (m.enable = function (I) {
              return (
                m.isEnabled ||
                  (tt(Pn ? Ee : g, "scroll", yo),
                  _.indexOf("scroll") >= 0 &&
                    tt(Pn ? Ee : g, "scroll", Xt, Fn, pt),
                  _.indexOf("wheel") >= 0 && tt(g, "wheel", jt, Fn, pt),
                  ((_.indexOf("touch") >= 0 && et) ||
                    _.indexOf("pointer") >= 0) &&
                    (tt(g, zt[0], In, Fn, pt),
                    tt(Ee, zt[2], K),
                    tt(Ee, zt[3], K),
                    Te && tt(g, "click", xi, !0, !0),
                    Gt && tt(g, "click", Ci),
                    Ae && tt(Ee, "gesturestart", $n),
                    u && tt(Ee, "gestureend", Vt),
                    Le && tt(g, de + "enter", ei),
                    st && tt(g, de + "leave", un),
                    ct && tt(g, de + "move", Gn)),
                  (m.isEnabled = !0),
                  (m.isDragging = m.isGesturing = m.isPressed = q = St = !1),
                  m._vx.reset(),
                  m._vy.reset(),
                  (Sn = ye()),
                  (kn = cn()),
                  I && I.type && In(I),
                  Ln && Ln(m)),
                m
              );
            }),
            (m.disable = function () {
              m.isEnabled &&
                (Vn.filter(function (I) {
                  return I !== m && pi(I.target);
                }).length || nt(Pn ? Ee : g, "scroll", yo),
                m.isPressed &&
                  (m._vx.reset(),
                  m._vy.reset(),
                  nt(Ce ? g : Ee, zt[1], On, !0)),
                nt(Pn ? Ee : g, "scroll", Xt, pt),
                nt(g, "wheel", jt, pt),
                nt(g, zt[0], In, pt),
                nt(Ee, zt[2], K),
                nt(Ee, zt[3], K),
                nt(g, "click", xi, !0),
                nt(g, "click", Ci),
                nt(Ee, "gesturestart", $n),
                nt(Ee, "gestureend", Vt),
                nt(g, de + "enter", ei),
                nt(g, de + "leave", un),
                nt(g, de + "move", Gn),
                (m.isEnabled = m.isPressed = m.isDragging = !1),
                fn && fn(m));
            }),
            (m.kill = m.revert =
              function () {
                m.disable();
                var I = Vn.indexOf(m);
                (I >= 0 && Vn.splice(I, 1), nn === m && (nn = 0));
              }),
            Vn.push(m),
            Ce && pi(g) && (nn = m),
            m.enable(Fe));
        }),
        i(d, [
          {
            key: "velocityX",
            get: function () {
              return this._vx.getVelocity();
            },
          },
          {
            key: "velocityY",
            get: function () {
              return this._vy.getVelocity();
            },
          },
        ]),
        d
      );
    })();
    ((Oe.version = "3.12.7"),
      (Oe.create = function (d) {
        return new Oe(d);
      }),
      (Oe.register = cr),
      (Oe.getAll = function () {
        return Vn.slice();
      }),
      (Oe.getById = function (d) {
        return Vn.filter(function (e) {
          return e.vars.id === d;
        })[0];
      }),
      rr() && a.registerPlugin(Oe));
    var v,
      Xn,
      z,
      ae,
      Ct,
      ee,
      bo,
      Ri,
      ui,
      gi,
      mi,
      Li,
      Ke,
      ki,
      xo,
      ot,
      pr,
      dr,
      jn,
      ur,
      Co,
      gr,
      rt,
      Ao,
      mr,
      _r,
      _n,
      To,
      wo,
      Kn,
      Do,
      Ii,
      Mo,
      No,
      $i = 1,
      Ye = Date.now,
      Ro = Ye(),
      Lt = 0,
      _i = 0,
      fr = function (e, o, t) {
        var n = At(e) && (e.substr(0, 6) === "clamp(" || e.indexOf("max") > -1);
        return ((t["_" + o + "Clamp"] = n), n ? e.substr(6, e.length - 7) : e);
      },
      hr = function (e, o) {
        return o && (!At(e) || e.substr(0, 6) !== "clamp(")
          ? "clamp(" + e + ")"
          : e;
      },
      Wa = function d() {
        return _i && requestAnimationFrame(d);
      },
      Fr = function () {
        return (ki = 1);
      },
      Sr = function () {
        return (ki = 0);
      },
      qt = function (e) {
        return e;
      },
      fi = function (e) {
        return Math.round(e * 1e5) / 1e5 || 0;
      },
      Pr = function () {
        return typeof window < "u";
      },
      Or = function () {
        return v || (Pr() && (v = window.gsap) && v.registerPlugin && v);
      },
      Cn = function (e) {
        return !!~bo.indexOf(e);
      },
      vr = function (e) {
        return (
          (e === "Height" ? Do : z["inner" + e]) ||
          Ct["client" + e] ||
          ee["client" + e]
        );
      },
      yr = function (e) {
        return (
          gn(e, "getBoundingClientRect") ||
          (Cn(e)
            ? function () {
                return ((Ji.width = z.innerWidth), (Ji.height = Do), Ji);
              }
            : function () {
                return on(e);
              })
        );
      },
      Va = function (e, o, t) {
        var n = t.d,
          s = t.d2,
          _ = t.a;
        return (_ = gn(e, "getBoundingClientRect"))
          ? function () {
              return _()[n];
            }
          : function () {
              return (o ? vr(s) : e["client" + s]) || 0;
            };
      },
      Xa = function (e, o) {
        return !o || ~Ut.indexOf(e)
          ? yr(e)
          : function () {
              return Ji;
            };
      },
      Jt = function (e, o) {
        var t = o.s,
          n = o.d2,
          s = o.d,
          _ = o.a;
        return Math.max(
          0,
          (t = "scroll" + n) && (_ = gn(e, t))
            ? _() - yr(e)()[s]
            : Cn(e)
              ? (Ct[t] || ee[t]) - vr(n)
              : e[t] - e["offset" + n],
        );
      },
      Gi = function (e, o) {
        for (var t = 0; t < jn.length; t += 3)
          (!o || ~o.indexOf(jn[t + 1])) && e(jn[t], jn[t + 1], jn[t + 2]);
      },
      At = function (e) {
        return typeof e == "string";
      },
      Ue = function (e) {
        return typeof e == "function";
      },
      hi = function (e) {
        return typeof e == "number";
      },
      An = function (e) {
        return typeof e == "object";
      },
      Fi = function (e, o, t) {
        return e && e.progress(o ? 0 : 1) && t && e.pause();
      },
      Lo = function (e, o) {
        if (e.enabled) {
          var t = e._ctx
            ? e._ctx.add(function () {
                return o(e);
              })
            : o(e);
          t && t.totalTime && (e.callbackAnimation = t);
        }
      },
      Yn = Math.abs,
      Er = "left",
      br = "top",
      ko = "right",
      Io = "bottom",
      Tn = "width",
      wn = "height",
      Si = "Right",
      Pi = "Left",
      Oi = "Top",
      vi = "Bottom",
      xe = "padding",
      kt = "margin",
      Un = "Width",
      $o = "Height",
      Ne = "px",
      It = function (e) {
        return z.getComputedStyle(e);
      },
      ja = function (e) {
        var o = It(e).position;
        e.style.position = o === "absolute" || o === "fixed" ? o : "relative";
      },
      xr = function (e, o) {
        for (var t in o) t in e || (e[t] = o[t]);
        return e;
      },
      on = function (e, o) {
        var t =
            o &&
            It(e)[xo] !== "matrix(1, 0, 0, 1, 0, 0)" &&
            v
              .to(e, {
                x: 0,
                y: 0,
                xPercent: 0,
                yPercent: 0,
                rotation: 0,
                rotationX: 0,
                rotationY: 0,
                scale: 1,
                skewX: 0,
                skewY: 0,
              })
              .progress(1),
          n = e.getBoundingClientRect();
        return (t && t.progress(0).kill(), n);
      },
      Bi = function (e, o) {
        var t = o.d2;
        return e["offset" + t] || e["client" + t] || 0;
      },
      Cr = function (e) {
        var o = [],
          t = e.labels,
          n = e.duration(),
          s;
        for (s in t) o.push(t[s] / n);
        return o;
      },
      Ka = function (e) {
        return function (o) {
          return v.utils.snap(Cr(e), o);
        };
      },
      Go = function (e) {
        var o = v.utils.snap(e),
          t =
            Array.isArray(e) &&
            e.slice(0).sort(function (n, s) {
              return n - s;
            });
        return t
          ? function (n, s, _) {
              _ === void 0 && (_ = 0.001);
              var g;
              if (!s) return o(n);
              if (s > 0) {
                for (n -= _, g = 0; g < t.length; g++)
                  if (t[g] >= n) return t[g];
                return t[g - 1];
              } else
                for (g = t.length, n += _; g--; ) if (t[g] <= n) return t[g];
              return t[0];
            }
          : function (n, s, _) {
              _ === void 0 && (_ = 0.001);
              var g = o(n);
              return !s || Math.abs(g - n) < _ || g - n < 0 == s < 0
                ? g
                : o(s < 0 ? n - e : n + e);
            };
      },
      Ya = function (e) {
        return function (o, t) {
          return Go(Cr(e))(o, t.direction);
        };
      },
      Hi = function (e, o, t, n) {
        return t.split(",").forEach(function (s) {
          return e(o, s, n);
        });
      },
      $e = function (e, o, t, n, s) {
        return e.addEventListener(o, t, { passive: !n, capture: !!s });
      },
      Ge = function (e, o, t, n) {
        return e.removeEventListener(o, t, !!n);
      },
      zi = function (e, o, t) {
        ((t = t && t.wheelHandler),
          t && (e(o, "wheel", t), e(o, "touchmove", t)));
      },
      Ar = {
        startColor: "green",
        endColor: "red",
        indent: 0,
        fontSize: "16px",
        fontWeight: "normal",
      },
      Wi = { toggleActions: "play", anticipatePin: 0 },
      Vi = { top: 0, left: 0, center: 0.5, bottom: 1, right: 1 },
      Xi = function (e, o) {
        if (At(e)) {
          var t = e.indexOf("="),
            n = ~t ? +(e.charAt(t - 1) + 1) * parseFloat(e.substr(t + 1)) : 0;
          (~t &&
            (e.indexOf("%") > t && (n *= o / 100), (e = e.substr(0, t - 1))),
            (e =
              n +
              (e in Vi
                ? Vi[e] * o
                : ~e.indexOf("%")
                  ? (parseFloat(e) * o) / 100
                  : parseFloat(e) || 0)));
        }
        return e;
      },
      ji = function (e, o, t, n, s, _, g, O) {
        var D = s.startColor,
          Q = s.endColor,
          j = s.fontSize,
          C = s.indent,
          h = s.fontWeight,
          T = ae.createElement("div"),
          Fe = Cn(t) || gn(t, "pinType") === "fixed",
          le = e.indexOf("scroller") !== -1,
          Re = Fe ? ee : t,
          fe = e.indexOf("start") !== -1,
          Ve = fe ? D : Q,
          $ =
            "border-color:" +
            Ve +
            ";font-size:" +
            j +
            ";color:" +
            Ve +
            ";font-weight:" +
            h +
            ";pointer-events:none;white-space:nowrap;font-family:sans-serif,Arial;z-index:1000;padding:4px 8px;border-width:0;border-style:solid;";
        return (
          ($ += "position:" + ((le || O) && Fe ? "fixed;" : "absolute;")),
          (le || O || !Fe) &&
            ($ += (n === Me ? ko : Io) + ":" + (_ + parseFloat(C)) + "px;"),
          g &&
            ($ +=
              "box-sizing:border-box;text-align:left;width:" +
              g.offsetWidth +
              "px;"),
          (T._isStart = fe),
          T.setAttribute(
            "class",
            "gsap-marker-" + e + (o ? " marker-" + o : ""),
          ),
          (T.style.cssText = $),
          (T.innerText = o || o === 0 ? e + "-" + o : e),
          Re.children[0]
            ? Re.insertBefore(T, Re.children[0])
            : Re.appendChild(T),
          (T._offset = T["offset" + n.op.d2]),
          Ki(T, 0, n, fe),
          T
        );
      },
      Ki = function (e, o, t, n) {
        var s = { display: "block" },
          _ = t[n ? "os2" : "p2"],
          g = t[n ? "p2" : "os2"];
        ((e._isFlipped = n),
          (s[t.a + "Percent"] = n ? -100 : 0),
          (s[t.a] = n ? "1px" : 0),
          (s["border" + _ + Un] = 1),
          (s["border" + g + Un] = 0),
          (s[t.p] = o + "px"),
          v.set(e, s));
      },
      G = [],
      Bo = {},
      yi,
      Tr = function () {
        return Ye() - Lt > 34 && (yi || (yi = requestAnimationFrame(rn)));
      },
      qn = function () {
        (!rt || !rt.isPressed || rt.startX > ee.clientWidth) &&
          (H.cache++,
          rt ? yi || (yi = requestAnimationFrame(rn)) : rn(),
          Lt || Mn("scrollStart"),
          (Lt = Ye()));
      },
      Ho = function () {
        ((_r = z.innerWidth), (mr = z.innerHeight));
      },
      Ei = function (e) {
        (H.cache++,
          (e === !0 ||
            (!Ke &&
              !gr &&
              !ae.fullscreenElement &&
              !ae.webkitFullscreenElement &&
              (!Ao ||
                _r !== z.innerWidth ||
                Math.abs(z.innerHeight - mr) > z.innerHeight * 0.25))) &&
            Ri.restart(!0));
      },
      Dn = {},
      Ua = [],
      wr = function d() {
        return Ge(W, "scrollEnd", d) || Rn(!0);
      },
      Mn = function (e) {
        return (
          (Dn[e] &&
            Dn[e].map(function (o) {
              return o();
            })) ||
          Ua
        );
      },
      Tt = [],
      Dr = function (e) {
        for (var o = 0; o < Tt.length; o += 5)
          (!e || (Tt[o + 4] && Tt[o + 4].query === e)) &&
            ((Tt[o].style.cssText = Tt[o + 1]),
            Tt[o].getBBox && Tt[o].setAttribute("transform", Tt[o + 2] || ""),
            (Tt[o + 3].uncache = 1));
      },
      zo = function (e, o) {
        var t;
        for (ot = 0; ot < G.length; ot++)
          ((t = G[ot]),
            t && (!o || t._ctx === o) && (e ? t.kill(1) : t.revert(!0, !0)));
        ((Ii = !0), o && Dr(o), o || Mn("revert"));
      },
      Mr = function (e, o) {
        (H.cache++,
          (o || !at) &&
            H.forEach(function (t) {
              return Ue(t) && t.cacheID++ && (t.rec = 0);
            }),
          At(e) && (z.history.scrollRestoration = wo = e));
      },
      at,
      Nn = 0,
      Nr,
      qa = function () {
        if (Nr !== Nn) {
          var e = (Nr = Nn);
          requestAnimationFrame(function () {
            return e === Nn && Rn(!0);
          });
        }
      },
      Rr = function () {
        (ee.appendChild(Kn),
          (Do = (!rt && Kn.offsetHeight) || z.innerHeight),
          ee.removeChild(Kn));
      },
      Lr = function (e) {
        return ui(
          ".gsap-marker-start, .gsap-marker-end, .gsap-marker-scroller-start, .gsap-marker-scroller-end",
        ).forEach(function (o) {
          return (o.style.display = e ? "none" : "block");
        });
      },
      Rn = function (e, o) {
        if (
          ((Ct = ae.documentElement),
          (ee = ae.body),
          (bo = [z, ae, Ct, ee]),
          Lt && !e && !Ii)
        ) {
          $e(W, "scrollEnd", wr);
          return;
        }
        (Rr(),
          (at = W.isRefreshing = !0),
          H.forEach(function (n) {
            return Ue(n) && ++n.cacheID && (n.rec = n());
          }));
        var t = Mn("refreshInit");
        (ur && W.sort(),
          o || zo(),
          H.forEach(function (n) {
            Ue(n) &&
              (n.smooth && (n.target.style.scrollBehavior = "auto"), n(0));
          }),
          G.slice(0).forEach(function (n) {
            return n.refresh();
          }),
          (Ii = !1),
          G.forEach(function (n) {
            if (n._subPinOffset && n.pin) {
              var s = n.vars.horizontal ? "offsetWidth" : "offsetHeight",
                _ = n.pin[s];
              (n.revert(!0, 1), n.adjustPinSpacing(n.pin[s] - _), n.refresh());
            }
          }),
          (Mo = 1),
          Lr(!0),
          G.forEach(function (n) {
            var s = Jt(n.scroller, n._dir),
              _ = n.vars.end === "max" || (n._endClamp && n.end > s),
              g = n._startClamp && n.start >= s;
            (_ || g) &&
              n.setPositions(
                g ? s - 1 : n.start,
                _ ? Math.max(g ? s : n.start + 1, s) : n.end,
                !0,
              );
          }),
          Lr(!1),
          (Mo = 0),
          t.forEach(function (n) {
            return n && n.render && n.render(-1);
          }),
          H.forEach(function (n) {
            Ue(n) &&
              (n.smooth &&
                requestAnimationFrame(function () {
                  return (n.target.style.scrollBehavior = "smooth");
                }),
              n.rec && n(n.rec));
          }),
          Mr(wo, 1),
          Ri.pause(),
          Nn++,
          (at = 2),
          rn(2),
          G.forEach(function (n) {
            return Ue(n.vars.onRefresh) && n.vars.onRefresh(n);
          }),
          (at = W.isRefreshing = !1),
          Mn("refresh"));
      },
      Wo = 0,
      Yi = 1,
      bi,
      rn = function (e) {
        if (e === 2 || (!at && !Ii)) {
          ((W.isUpdating = !0), bi && bi.update(0));
          var o = G.length,
            t = Ye(),
            n = t - Ro >= 50,
            s = o && G[0].scroll();
          if (
            ((Yi = Wo > s ? -1 : 1),
            at || (Wo = s),
            n &&
              (Lt && !ki && t - Lt > 200 && ((Lt = 0), Mn("scrollEnd")),
              (mi = Ro),
              (Ro = t)),
            Yi < 0)
          ) {
            for (ot = o; ot-- > 0; ) G[ot] && G[ot].update(0, n);
            Yi = 1;
          } else for (ot = 0; ot < o; ot++) G[ot] && G[ot].update(0, n);
          W.isUpdating = !1;
        }
        yi = 0;
      },
      Vo = [
        Er,
        br,
        Io,
        ko,
        kt + vi,
        kt + Si,
        kt + Oi,
        kt + Pi,
        "display",
        "flexShrink",
        "float",
        "zIndex",
        "gridColumnStart",
        "gridColumnEnd",
        "gridRowStart",
        "gridRowEnd",
        "gridArea",
        "justifySelf",
        "alignSelf",
        "placeSelf",
        "order",
      ],
      Ui = Vo.concat([
        Tn,
        wn,
        "boxSizing",
        "max" + Un,
        "max" + $o,
        "position",
        kt,
        xe,
        xe + Oi,
        xe + Si,
        xe + vi,
        xe + Pi,
      ]),
      Ja = function (e, o, t) {
        Jn(t);
        var n = e._gsap;
        if (n.spacerIsNative) Jn(n.spacerState);
        else if (e._gsap.swappedIn) {
          var s = o.parentNode;
          s && (s.insertBefore(e, o), s.removeChild(o));
        }
        e._gsap.swappedIn = !1;
      },
      Xo = function (e, o, t, n) {
        if (!e._gsap.swappedIn) {
          for (var s = Vo.length, _ = o.style, g = e.style, O; s--; )
            ((O = Vo[s]), (_[O] = t[O]));
          ((_.position = t.position === "absolute" ? "absolute" : "relative"),
            t.display === "inline" && (_.display = "inline-block"),
            (g[Io] = g[ko] = "auto"),
            (_.flexBasis = t.flexBasis || "auto"),
            (_.overflow = "visible"),
            (_.boxSizing = "border-box"),
            (_[Tn] = Bi(e, it) + Ne),
            (_[wn] = Bi(e, Me) + Ne),
            (_[xe] = g[kt] = g[br] = g[Er] = "0"),
            Jn(n),
            (g[Tn] = g["max" + Un] = t[Tn]),
            (g[wn] = g["max" + $o] = t[wn]),
            (g[xe] = t[xe]),
            e.parentNode !== o &&
              (e.parentNode.insertBefore(o, e), o.appendChild(e)),
            (e._gsap.swappedIn = !0));
        }
      },
      Qa = /([A-Z])/g,
      Jn = function (e) {
        if (e) {
          var o = e.t.style,
            t = e.length,
            n = 0,
            s,
            _;
          for ((e.t._gsap || v.core.getCache(e.t)).uncache = 1; n < t; n += 2)
            ((_ = e[n + 1]),
              (s = e[n]),
              _
                ? (o[s] = _)
                : o[s] && o.removeProperty(s.replace(Qa, "-$1").toLowerCase()));
        }
      },
      qi = function (e) {
        for (var o = Ui.length, t = e.style, n = [], s = 0; s < o; s++)
          n.push(Ui[s], t[Ui[s]]);
        return ((n.t = e), n);
      },
      Za = function (e, o, t) {
        for (var n = [], s = e.length, _ = t ? 8 : 0, g; _ < s; _ += 2)
          ((g = e[_]), n.push(g, g in o ? o[g] : e[_ + 1]));
        return ((n.t = e.t), n);
      },
      Ji = { left: 0, top: 0 },
      kr = function (e, o, t, n, s, _, g, O, D, Q, j, C, h, T) {
        (Ue(e) && (e = e(O)),
          At(e) &&
            e.substr(0, 3) === "max" &&
            (e = C + (e.charAt(4) === "=" ? Xi("0" + e.substr(3), t) : 0)));
        var Fe = h ? h.time() : 0,
          le,
          Re,
          fe;
        if ((h && h.seek(0), isNaN(e) || (e = +e), hi(e)))
          (h &&
            (e = v.utils.mapRange(
              h.scrollTrigger.start,
              h.scrollTrigger.end,
              0,
              C,
              e,
            )),
            g && Ki(g, t, n, !0));
        else {
          Ue(o) && (o = o(O));
          var Ve = (e || "0").split(" "),
            $,
            $t,
            ge,
            R;
          ((fe = ht(o, O) || ee),
            ($ = on(fe) || {}),
            (!$ || (!$.left && !$.top)) &&
              It(fe).display === "none" &&
              ((R = fe.style.display),
              (fe.style.display = "block"),
              ($ = on(fe)),
              R ? (fe.style.display = R) : fe.style.removeProperty("display")),
            ($t = Xi(Ve[0], $[n.d])),
            (ge = Xi(Ve[1] || "0", t)),
            (e = $[n.p] - D[n.p] - Q + $t + s - ge),
            g && Ki(g, ge, n, t - ge < 20 || (g._isStart && ge > 20)),
            (t -= t - ge));
        }
        if ((T && ((O[T] = e || -0.001), e < 0 && (e = 0)), _)) {
          var lt = e + t,
            Ft = _._isStart;
          ((le = "scroll" + n.d2),
            Ki(
              _,
              lt,
              n,
              (Ft && lt > 20) ||
                (!Ft &&
                  (j ? Math.max(ee[le], Ct[le]) : _.parentNode[le]) <= lt + 1),
            ),
            j &&
              ((D = on(g)),
              j && (_.style[n.op.p] = D[n.op.p] - n.op.m - _._offset + Ne)));
        }
        return (
          h &&
            fe &&
            ((le = on(fe)),
            h.seek(C),
            (Re = on(fe)),
            (h._caScrollDist = le[n.p] - Re[n.p]),
            (e = (e / h._caScrollDist) * C)),
          h && h.seek(Fe),
          h ? e : Math.round(e)
        );
      },
      el = /(webkit|moz|length|cssText|inset)/i,
      Ir = function (e, o, t, n) {
        if (e.parentNode !== o) {
          var s = e.style,
            _,
            g;
          if (o === ee) {
            ((e._stOrig = s.cssText), (g = It(e)));
            for (_ in g)
              !+_ &&
                !el.test(_) &&
                g[_] &&
                typeof s[_] == "string" &&
                _ !== "0" &&
                (s[_] = g[_]);
            ((s.top = t), (s.left = n));
          } else s.cssText = e._stOrig;
          ((v.core.getCache(e).uncache = 1), o.appendChild(e));
        }
      },
      $r = function (e, o, t) {
        var n = o,
          s = n;
        return function (_) {
          var g = Math.round(e());
          return (
            g !== n &&
              g !== s &&
              Math.abs(g - n) > 3 &&
              Math.abs(g - s) > 3 &&
              ((_ = g), t && t()),
            (s = n),
            (n = Math.round(_)),
            n
          );
        };
      },
      Qi = function (e, o, t) {
        var n = {};
        ((n[o.p] = "+=" + t), v.set(e, n));
      },
      Gr = function (e, o) {
        var t = mn(e, o),
          n = "_scroll" + o.p2,
          s = function _(g, O, D, Q, j) {
            var C = _.tween,
              h = O.onComplete,
              T = {};
            D = D || t();
            var Fe = $r(t, D, function () {
              (C.kill(), (_.tween = 0));
            });
            return (
              (j = (Q && j) || 0),
              (Q = Q || g - D),
              C && C.kill(),
              (O[n] = g),
              (O.inherit = !1),
              (O.modifiers = T),
              (T[n] = function () {
                return Fe(D + Q * C.ratio + j * C.ratio * C.ratio);
              }),
              (O.onUpdate = function () {
                (H.cache++, _.tween && rn());
              }),
              (O.onComplete = function () {
                ((_.tween = 0), h && h.call(C));
              }),
              (C = _.tween = v.to(e, O)),
              C
            );
          };
        return (
          (e[n] = t),
          (t.wheelHandler = function () {
            return s.tween && s.tween.kill() && (s.tween = 0);
          }),
          $e(e, "wheel", t.wheelHandler),
          W.isTouch && $e(e, "touchmove", t.wheelHandler),
          s
        );
      },
      W = (function () {
        function d(o, t) {
          (Xn ||
            d.register(v) ||
            console.warn("Please gsap.registerPlugin(ScrollTrigger)"),
            To(this),
            this.init(o, t));
        }
        var e = d.prototype;
        return (
          (e.init = function (t, n) {
            if (
              ((this.progress = this.start = 0),
              this.vars && this.kill(!0, !0),
              !_i)
            ) {
              this.update = this.refresh = this.kill = qt;
              return;
            }
            t = xr(At(t) || hi(t) || t.nodeType ? { trigger: t } : t, Wi);
            var s = t,
              _ = s.onUpdate,
              g = s.toggleClass,
              O = s.id,
              D = s.onToggle,
              Q = s.onRefresh,
              j = s.scrub,
              C = s.trigger,
              h = s.pin,
              T = s.pinSpacing,
              Fe = s.invalidateOnRefresh,
              le = s.anticipatePin,
              Re = s.onScrubComplete,
              fe = s.onSnapComplete,
              Ve = s.once,
              $ = s.snap,
              $t = s.pinReparent,
              ge = s.pinSpacer,
              R = s.containerAnimation,
              lt = s.fastScrollEnd,
              Ft = s.preventOverlaps,
              b =
                t.horizontal || (t.containerAnimation && t.horizontal !== !1)
                  ? it
                  : Me,
              Be = !j && j !== 0,
              N = ht(t.scroller || z),
              an = v.core.getCache(N),
              Le = Cn(N),
              st =
                ("pinType" in t
                  ? t.pinType
                  : gn(N, "pinType") || (Le && "fixed")) === "fixed",
              ct = [t.onEnter, t.onLeave, t.onEnterBack, t.onLeaveBack],
              oe = Be && t.toggleActions.split(" "),
              Ce = "markers" in t ? t.markers : Wi.markers,
              Ae = Le ? 0 : parseFloat(It(N)["border" + b.p2 + Un]) || 0,
              u = this,
              ke =
                t.onRefreshInit &&
                function () {
                  return t.onRefreshInit(u);
                },
              Ln = Va(N, Le, b),
              fn = Xa(N, Le),
              Gt = 0,
              ln = 0,
              pt = 0,
              Te = mn(N, b),
              dt,
              qe,
              hn,
              ut,
              St,
              q,
              ve,
              Pt,
              wt,
              m,
              Dt,
              sn,
              Fn,
              ye,
              cn,
              Sn,
              kn,
              He,
              Pn,
              Ee,
              Wt,
              Bt,
              pn,
              xi,
              we,
              eo,
              dn,
              Qn,
              Zn,
              On,
              In,
              K,
              $n,
              Vt,
              Xt,
              jt,
              Gn,
              ei,
              un;
            if (
              ((u._startClamp = u._endClamp = !1),
              (u._dir = b),
              (le *= 45),
              (u.scroller = N),
              (u.scroll = R ? R.time.bind(R) : Te),
              (ut = Te()),
              (u.vars = t),
              (n = n || t.animation),
              "refreshPriority" in t &&
                ((ur = 1), t.refreshPriority === -9999 && (bi = u)),
              (an.tweenScroll = an.tweenScroll || {
                top: Gr(N, Me),
                left: Gr(N, it),
              }),
              (u.tweenTo = dt = an.tweenScroll[b.p]),
              (u.scrubDuration = function (P) {
                (($n = hi(P) && P),
                  $n
                    ? K
                      ? K.duration(P)
                      : (K = v.to(n, {
                          ease: "expo",
                          totalProgress: "+=0",
                          inherit: !1,
                          duration: $n,
                          paused: !0,
                          onComplete: function () {
                            return Re && Re(u);
                          },
                        }))
                    : (K && K.progress(1).kill(), (K = 0)));
              }),
              n &&
                ((n.vars.lazy = !1),
                (n._initted && !u.isReverted) ||
                  (n.vars.immediateRender !== !1 &&
                    t.immediateRender !== !1 &&
                    n.duration() &&
                    n.render(0, !0, !0)),
                (u.animation = n.pause()),
                (n.scrollTrigger = u),
                u.scrubDuration(j),
                (On = 0),
                O || (O = n.vars.id)),
              $ &&
                ((!An($) || $.push) && ($ = { snapTo: $ }),
                "scrollBehavior" in ee.style &&
                  v.set(Le ? [ee, Ct] : N, { scrollBehavior: "auto" }),
                H.forEach(function (P) {
                  return (
                    Ue(P) &&
                    P.target === (Le ? ae.scrollingElement || Ct : N) &&
                    (P.smooth = !1)
                  );
                }),
                (hn = Ue($.snapTo)
                  ? $.snapTo
                  : $.snapTo === "labels"
                    ? Ka(n)
                    : $.snapTo === "labelsDirectional"
                      ? Ya(n)
                      : $.directional !== !1
                        ? function (P, L) {
                            return Go($.snapTo)(
                              P,
                              Ye() - ln < 500 ? 0 : L.direction,
                            );
                          }
                        : v.utils.snap($.snapTo)),
                (Vt = $.duration || { min: 0.1, max: 2 }),
                (Vt = An(Vt) ? gi(Vt.min, Vt.max) : gi(Vt, Vt)),
                (Xt = v
                  .delayedCall($.delay || $n / 2 || 0.1, function () {
                    var P = Te(),
                      L = Ye() - ln < 500,
                      A = dt.tween;
                    if (
                      (L || Math.abs(u.getVelocity()) < 10) &&
                      !A &&
                      !ki &&
                      Gt !== P
                    ) {
                      var k = (P - q) / ye,
                        ze = n && !Be ? n.totalProgress() : k,
                        U = L ? 0 : ((ze - In) / (Ye() - mi)) * 1e3 || 0,
                        be = v.utils.clamp(-k, 1 - k, (Yn(U / 2) * U) / 0.185),
                        Je = k + ($.inertia === !1 ? 0 : be),
                        Se,
                        me,
                        re = $,
                        Kt = re.onStart,
                        he = re.onInterrupt,
                        Mt = re.onComplete;
                      if (
                        ((Se = hn(Je, u)),
                        hi(Se) || (Se = Je),
                        (me = Math.max(0, Math.round(q + Se * ye))),
                        P <= ve && P >= q && me !== P)
                      ) {
                        if (A && !A._initted && A.data <= Yn(me - P)) return;
                        ($.inertia === !1 && (be = Se - k),
                          dt(
                            me,
                            {
                              duration: Vt(
                                Yn(
                                  (Math.max(Yn(Je - ze), Yn(Se - ze)) * 0.185) /
                                    U /
                                    0.05 || 0,
                                ),
                              ),
                              ease: $.ease || "power3",
                              data: Yn(me - P),
                              onInterrupt: function () {
                                return Xt.restart(!0) && he && he(u);
                              },
                              onComplete: function () {
                                (u.update(),
                                  (Gt = Te()),
                                  n &&
                                    !Be &&
                                    (K
                                      ? K.resetTo(
                                          "totalProgress",
                                          Se,
                                          n._tTime / n._tDur,
                                        )
                                      : n.progress(Se)),
                                  (On = In =
                                    n && !Be ? n.totalProgress() : u.progress),
                                  fe && fe(u),
                                  Mt && Mt(u));
                              },
                            },
                            P,
                            be * ye,
                            me - P - be * ye,
                          ),
                          Kt && Kt(u, dt.tween));
                      }
                    } else u.isActive && Gt !== P && Xt.restart(!0);
                  })
                  .pause())),
              O && (Bo[O] = u),
              (C = u.trigger = ht(C || (h !== !0 && h))),
              (un = C && C._gsap && C._gsap.stRevert),
              un && (un = un(u)),
              (h = h === !0 ? C : ht(h)),
              At(g) && (g = { targets: C, className: g }),
              h &&
                (T === !1 ||
                  T === kt ||
                  (T =
                    !T &&
                    h.parentNode &&
                    h.parentNode.style &&
                    It(h.parentNode).display === "flex"
                      ? !1
                      : xe),
                (u.pin = h),
                (qe = v.core.getCache(h)),
                qe.spacer
                  ? (cn = qe.pinState)
                  : (ge &&
                      ((ge = ht(ge)),
                      ge &&
                        !ge.nodeType &&
                        (ge = ge.current || ge.nativeElement),
                      (qe.spacerIsNative = !!ge),
                      ge && (qe.spacerState = qi(ge))),
                    (qe.spacer = He = ge || ae.createElement("div")),
                    He.classList.add("pin-spacer"),
                    O && He.classList.add("pin-spacer-" + O),
                    (qe.pinState = cn = qi(h))),
                t.force3D !== !1 && v.set(h, { force3D: !0 }),
                (u.spacer = He = qe.spacer),
                (Zn = It(h)),
                (xi = Zn[T + b.os2]),
                (Ee = v.getProperty(h)),
                (Wt = v.quickSetter(h, b.a, Ne)),
                Xo(h, He, Zn),
                (kn = qi(h))),
              Ce)
            ) {
              ((sn = An(Ce) ? xr(Ce, Ar) : Ar),
                (m = ji("scroller-start", O, N, b, sn, 0)),
                (Dt = ji("scroller-end", O, N, b, sn, 0, m)),
                (Pn = m["offset" + b.op.d2]));
              var Ci = ht(gn(N, "content") || N);
              ((Pt = this.markerStart = ji("start", O, Ci, b, sn, Pn, 0, R)),
                (wt = this.markerEnd = ji("end", O, Ci, b, sn, Pn, 0, R)),
                R && (ei = v.quickSetter([Pt, wt], b.a, Ne)),
                !st &&
                  !(Ut.length && gn(N, "fixedMarkers") === !0) &&
                  (ja(Le ? ee : N),
                  v.set([m, Dt], { force3D: !0 }),
                  (eo = v.quickSetter(m, b.a, Ne)),
                  (Qn = v.quickSetter(Dt, b.a, Ne))));
            }
            if (R) {
              var I = R.vars.onUpdate,
                w = R.vars.onUpdateParams;
              R.eventCallback("onUpdate", function () {
                (u.update(0, 0, 1), I && I.apply(R, w || []));
              });
            }
            if (
              ((u.previous = function () {
                return G[G.indexOf(u) - 1];
              }),
              (u.next = function () {
                return G[G.indexOf(u) + 1];
              }),
              (u.revert = function (P, L) {
                if (!L) return u.kill(!0);
                var A = P !== !1 || !u.enabled,
                  k = Ke;
                A !== u.isReverted &&
                  (A &&
                    ((jt = Math.max(Te(), u.scroll.rec || 0)),
                    (pt = u.progress),
                    (Gn = n && n.progress())),
                  Pt &&
                    [Pt, wt, m, Dt].forEach(function (ze) {
                      return (ze.style.display = A ? "none" : "block");
                    }),
                  A && ((Ke = u), u.update(A)),
                  h &&
                    (!$t || !u.isActive) &&
                    (A ? Ja(h, He, cn) : Xo(h, He, It(h), we)),
                  A || u.update(A),
                  (Ke = k),
                  (u.isReverted = A));
              }),
              (u.refresh = function (P, L, A, k) {
                if (!((Ke || !u.enabled) && !L)) {
                  if (h && P && Lt) {
                    $e(d, "scrollEnd", wr);
                    return;
                  }
                  (!at && ke && ke(u),
                    (Ke = u),
                    dt.tween && !A && (dt.tween.kill(), (dt.tween = 0)),
                    K && K.pause(),
                    Fe && n && n.revert({ kill: !1 }).invalidate(),
                    u.isReverted || u.revert(!0, !0),
                    (u._subPinOffset = !1));
                  var ze = Ln(),
                    U = fn(),
                    be = R ? R.duration() : Jt(N, b),
                    Je = ye <= 0.01,
                    Se = 0,
                    me = k || 0,
                    re = An(A) ? A.end : t.end,
                    Kt = t.endTrigger || C,
                    he = An(A)
                      ? A.start
                      : t.start ||
                        (t.start === 0 || !C ? 0 : h ? "0 0" : "0 100%"),
                    Mt = (u.pinnedContainer =
                      t.pinnedContainer && ht(t.pinnedContainer, u)),
                    Qt = (C && Math.max(0, G.indexOf(u))) || 0,
                    Xe = Qt,
                    je,
                    Qe,
                    Bn,
                    to,
                    Ze,
                    Ie,
                    Zt,
                    Ko,
                    Vr,
                    Ai,
                    en,
                    Ti,
                    no;
                  for (
                    Ce &&
                    An(A) &&
                    ((Ti = v.getProperty(m, b.p)),
                    (no = v.getProperty(Dt, b.p)));
                    Xe-- > 0;
                  )
                    ((Ie = G[Xe]),
                      Ie.end || Ie.refresh(0, 1) || (Ke = u),
                      (Zt = Ie.pin),
                      Zt &&
                        (Zt === C || Zt === h || Zt === Mt) &&
                        !Ie.isReverted &&
                        (Ai || (Ai = []), Ai.unshift(Ie), Ie.revert(!0, !0)),
                      Ie !== G[Xe] && (Qt--, Xe--));
                  for (
                    Ue(he) && (he = he(u)),
                      he = fr(he, "start", u),
                      q =
                        kr(
                          he,
                          C,
                          ze,
                          b,
                          Te(),
                          Pt,
                          m,
                          u,
                          U,
                          Ae,
                          st,
                          be,
                          R,
                          u._startClamp && "_startClamp",
                        ) || (h ? -0.001 : 0),
                      Ue(re) && (re = re(u)),
                      At(re) &&
                        !re.indexOf("+=") &&
                        (~re.indexOf(" ")
                          ? (re = (At(he) ? he.split(" ")[0] : "") + re)
                          : ((Se = Xi(re.substr(2), ze)),
                            (re = At(he)
                              ? he
                              : (R
                                  ? v.utils.mapRange(
                                      0,
                                      R.duration(),
                                      R.scrollTrigger.start,
                                      R.scrollTrigger.end,
                                      q,
                                    )
                                  : q) + Se),
                            (Kt = C))),
                      re = fr(re, "end", u),
                      ve =
                        Math.max(
                          q,
                          kr(
                            re || (Kt ? "100% 0" : be),
                            Kt,
                            ze,
                            b,
                            Te() + Se,
                            wt,
                            Dt,
                            u,
                            U,
                            Ae,
                            st,
                            be,
                            R,
                            u._endClamp && "_endClamp",
                          ),
                        ) || -0.001,
                      Se = 0,
                      Xe = Qt;
                    Xe--;
                  )
                    ((Ie = G[Xe]),
                      (Zt = Ie.pin),
                      Zt &&
                        Ie.start - Ie._pinPush <= q &&
                        !R &&
                        Ie.end > 0 &&
                        ((je =
                          Ie.end -
                          (u._startClamp ? Math.max(0, Ie.start) : Ie.start)),
                        ((Zt === C && Ie.start - Ie._pinPush < q) ||
                          Zt === Mt) &&
                          isNaN(he) &&
                          (Se += je * (1 - Ie.progress)),
                        Zt === h && (me += je)));
                  if (
                    ((q += Se),
                    (ve += Se),
                    u._startClamp && (u._startClamp += Se),
                    u._endClamp &&
                      !at &&
                      ((u._endClamp = ve || -0.001),
                      (ve = Math.min(ve, Jt(N, b)))),
                    (ye = ve - q || ((q -= 0.01) && 0.001)),
                    Je &&
                      (pt = v.utils.clamp(0, 1, v.utils.normalize(q, ve, jt))),
                    (u._pinPush = me),
                    Pt &&
                      Se &&
                      ((je = {}),
                      (je[b.a] = "+=" + Se),
                      Mt && (je[b.p] = "-=" + Te()),
                      v.set([Pt, wt], je)),
                    h && !(Mo && u.end >= Jt(N, b)))
                  )
                    ((je = It(h)),
                      (to = b === Me),
                      (Bn = Te()),
                      (Bt = parseFloat(Ee(b.a)) + me),
                      !be &&
                        ve > 1 &&
                        ((en = (Le ? ae.scrollingElement || Ct : N).style),
                        (en = {
                          style: en,
                          value: en["overflow" + b.a.toUpperCase()],
                        }),
                        Le &&
                          It(ee)["overflow" + b.a.toUpperCase()] !== "scroll" &&
                          (en.style["overflow" + b.a.toUpperCase()] =
                            "scroll")),
                      Xo(h, He, je),
                      (kn = qi(h)),
                      (Qe = on(h, !0)),
                      (Ko = st && mn(N, to ? it : Me)()),
                      T
                        ? ((we = [T + b.os2, ye + me + Ne]),
                          (we.t = He),
                          (Xe = T === xe ? Bi(h, b) + ye + me : 0),
                          Xe &&
                            (we.push(b.d, Xe + Ne),
                            He.style.flexBasis !== "auto" &&
                              (He.style.flexBasis = Xe + Ne)),
                          Jn(we),
                          Mt &&
                            G.forEach(function (wi) {
                              wi.pin === Mt &&
                                wi.vars.pinSpacing !== !1 &&
                                (wi._subPinOffset = !0);
                            }),
                          st && Te(jt))
                        : ((Xe = Bi(h, b)),
                          Xe &&
                            He.style.flexBasis !== "auto" &&
                            (He.style.flexBasis = Xe + Ne)),
                      st &&
                        ((Ze = {
                          top: Qe.top + (to ? Bn - q : Ko) + Ne,
                          left: Qe.left + (to ? Ko : Bn - q) + Ne,
                          boxSizing: "border-box",
                          position: "fixed",
                        }),
                        (Ze[Tn] = Ze["max" + Un] = Math.ceil(Qe.width) + Ne),
                        (Ze[wn] = Ze["max" + $o] = Math.ceil(Qe.height) + Ne),
                        (Ze[kt] =
                          Ze[kt + Oi] =
                          Ze[kt + Si] =
                          Ze[kt + vi] =
                          Ze[kt + Pi] =
                            "0"),
                        (Ze[xe] = je[xe]),
                        (Ze[xe + Oi] = je[xe + Oi]),
                        (Ze[xe + Si] = je[xe + Si]),
                        (Ze[xe + vi] = je[xe + vi]),
                        (Ze[xe + Pi] = je[xe + Pi]),
                        (Sn = Za(cn, Ze, $t)),
                        at && Te(0)),
                      n
                        ? ((Vr = n._initted),
                          Co(1),
                          n.render(n.duration(), !0, !0),
                          (pn = Ee(b.a) - Bt + ye + me),
                          (dn = Math.abs(ye - pn) > 1),
                          st && dn && Sn.splice(Sn.length - 2, 2),
                          n.render(0, !0, !0),
                          Vr || n.invalidate(!0),
                          n.parent || n.totalTime(n.totalTime()),
                          Co(0))
                        : (pn = ye),
                      en &&
                        (en.value
                          ? (en.style["overflow" + b.a.toUpperCase()] =
                              en.value)
                          : en.style.removeProperty("overflow-" + b.a)));
                  else if (C && Te() && !R)
                    for (Qe = C.parentNode; Qe && Qe !== ee; )
                      (Qe._pinOffset &&
                        ((q -= Qe._pinOffset), (ve -= Qe._pinOffset)),
                        (Qe = Qe.parentNode));
                  (Ai &&
                    Ai.forEach(function (wi) {
                      return wi.revert(!1, !0);
                    }),
                    (u.start = q),
                    (u.end = ve),
                    (ut = St = at ? jt : Te()),
                    !R && !at && (ut < jt && Te(jt), (u.scroll.rec = 0)),
                    u.revert(!1, !0),
                    (ln = Ye()),
                    Xt && ((Gt = -1), Xt.restart(!0)),
                    (Ke = 0),
                    n &&
                      Be &&
                      (n._initted || Gn) &&
                      n.progress() !== Gn &&
                      n.progress(Gn || 0, !0).render(n.time(), !0, !0),
                    (Je ||
                      pt !== u.progress ||
                      R ||
                      Fe ||
                      (n && !n._initted)) &&
                      (n &&
                        !Be &&
                        n.totalProgress(
                          R && q < -0.001 && !pt
                            ? v.utils.normalize(q, ve, 0)
                            : pt,
                          !0,
                        ),
                      (u.progress = Je || (ut - q) / ye === pt ? 0 : pt)),
                    h && T && (He._pinOffset = Math.round(u.progress * pn)),
                    K && K.invalidate(),
                    isNaN(Ti) ||
                      ((Ti -= v.getProperty(m, b.p)),
                      (no -= v.getProperty(Dt, b.p)),
                      Qi(m, b, Ti),
                      Qi(Pt, b, Ti - (k || 0)),
                      Qi(Dt, b, no),
                      Qi(wt, b, no - (k || 0))),
                    Je && !at && u.update(),
                    Q && !at && !Fn && ((Fn = !0), Q(u), (Fn = !1)));
                }
              }),
              (u.getVelocity = function () {
                return ((Te() - St) / (Ye() - mi)) * 1e3 || 0;
              }),
              (u.endAnimation = function () {
                (Fi(u.callbackAnimation),
                  n &&
                    (K
                      ? K.progress(1)
                      : n.paused()
                        ? Be || Fi(n, u.direction < 0, 1)
                        : Fi(n, n.reversed())));
              }),
              (u.labelToScroll = function (P) {
                return (
                  (n &&
                    n.labels &&
                    (q || u.refresh() || q) +
                      (n.labels[P] / n.duration()) * ye) ||
                  0
                );
              }),
              (u.getTrailing = function (P) {
                var L = G.indexOf(u),
                  A =
                    u.direction > 0 ? G.slice(0, L).reverse() : G.slice(L + 1);
                return (
                  At(P)
                    ? A.filter(function (k) {
                        return k.vars.preventOverlaps === P;
                      })
                    : A
                ).filter(function (k) {
                  return u.direction > 0 ? k.end <= q : k.start >= ve;
                });
              }),
              (u.update = function (P, L, A) {
                if (!(R && !A && !P)) {
                  var k = at === !0 ? jt : u.scroll(),
                    ze = P ? 0 : (k - q) / ye,
                    U = ze < 0 ? 0 : ze > 1 ? 1 : ze || 0,
                    be = u.progress,
                    Je,
                    Se,
                    me,
                    re,
                    Kt,
                    he,
                    Mt,
                    Qt;
                  if (
                    (L &&
                      ((St = ut),
                      (ut = R ? Te() : k),
                      $ &&
                        ((In = On), (On = n && !Be ? n.totalProgress() : U))),
                    le &&
                      h &&
                      !Ke &&
                      !$i &&
                      Lt &&
                      (!U && q < k + ((k - St) / (Ye() - mi)) * le
                        ? (U = 1e-4)
                        : U === 1 &&
                          ve > k + ((k - St) / (Ye() - mi)) * le &&
                          (U = 0.9999)),
                    U !== be && u.enabled)
                  ) {
                    if (
                      ((Je = u.isActive = !!U && U < 1),
                      (Se = !!be && be < 1),
                      (he = Je !== Se),
                      (Kt = he || !!U != !!be),
                      (u.direction = U > be ? 1 : -1),
                      (u.progress = U),
                      Kt &&
                        !Ke &&
                        ((me = U && !be ? 0 : U === 1 ? 1 : be === 1 ? 2 : 3),
                        Be &&
                          ((re =
                            (!he && oe[me + 1] !== "none" && oe[me + 1]) ||
                            oe[me]),
                          (Qt =
                            n &&
                            (re === "complete" || re === "reset" || re in n)))),
                      Ft &&
                        (he || Qt) &&
                        (Qt || j || !n) &&
                        (Ue(Ft)
                          ? Ft(u)
                          : u.getTrailing(Ft).forEach(function (Bn) {
                              return Bn.endAnimation();
                            })),
                      Be ||
                        (K && !Ke && !$i
                          ? (K._dp._time - K._start !== K._time &&
                              K.render(K._dp._time - K._start),
                            K.resetTo
                              ? K.resetTo(
                                  "totalProgress",
                                  U,
                                  n._tTime / n._tDur,
                                )
                              : ((K.vars.totalProgress = U),
                                K.invalidate().restart()))
                          : n && n.totalProgress(U, !!(Ke && (ln || P)))),
                      h)
                    ) {
                      if ((P && T && (He.style[T + b.os2] = xi), !st))
                        Wt(fi(Bt + pn * U));
                      else if (Kt) {
                        if (
                          ((Mt =
                            !P && U > be && ve + 1 > k && k + 1 >= Jt(N, b)),
                          $t)
                        )
                          if (!P && (Je || Mt)) {
                            var Xe = on(h, !0),
                              je = k - q;
                            Ir(
                              h,
                              ee,
                              Xe.top + (b === Me ? je : 0) + Ne,
                              Xe.left + (b === Me ? 0 : je) + Ne,
                            );
                          } else Ir(h, He);
                        (Jn(Je || Mt ? Sn : kn),
                          (dn && U < 1 && Je) ||
                            Wt(Bt + (U === 1 && !Mt ? pn : 0)));
                      }
                    }
                    ($ && !dt.tween && !Ke && !$i && Xt.restart(!0),
                      g &&
                        (he || (Ve && U && (U < 1 || !No))) &&
                        ui(g.targets).forEach(function (Bn) {
                          return Bn.classList[Je || Ve ? "add" : "remove"](
                            g.className,
                          );
                        }),
                      _ && !Be && !P && _(u),
                      Kt && !Ke
                        ? (Be &&
                            (Qt &&
                              (re === "complete"
                                ? n.pause().totalProgress(1)
                                : re === "reset"
                                  ? n.restart(!0).pause()
                                  : re === "restart"
                                    ? n.restart(!0)
                                    : n[re]()),
                            _ && _(u)),
                          (he || !No) &&
                            (D && he && Lo(u, D),
                            ct[me] && Lo(u, ct[me]),
                            Ve && (U === 1 ? u.kill(!1, 1) : (ct[me] = 0)),
                            he ||
                              ((me = U === 1 ? 1 : 3),
                              ct[me] && Lo(u, ct[me]))),
                          lt &&
                            !Je &&
                            Math.abs(u.getVelocity()) > (hi(lt) ? lt : 2500) &&
                            (Fi(u.callbackAnimation),
                            K
                              ? K.progress(1)
                              : Fi(n, re === "reverse" ? 1 : !U, 1)))
                        : Be && _ && !Ke && _(u));
                  }
                  if (Qn) {
                    var Qe = R
                      ? (k / R.duration()) * (R._caScrollDist || 0)
                      : k;
                    (eo(Qe + (m._isFlipped ? 1 : 0)), Qn(Qe));
                  }
                  ei && ei((-k / R.duration()) * (R._caScrollDist || 0));
                }
              }),
              (u.enable = function (P, L) {
                u.enabled ||
                  ((u.enabled = !0),
                  $e(N, "resize", Ei),
                  Le || $e(N, "scroll", qn),
                  ke && $e(d, "refreshInit", ke),
                  P !== !1 && ((u.progress = pt = 0), (ut = St = Gt = Te())),
                  L !== !1 && u.refresh());
              }),
              (u.getTween = function (P) {
                return P && dt ? dt.tween : K;
              }),
              (u.setPositions = function (P, L, A, k) {
                if (R) {
                  var ze = R.scrollTrigger,
                    U = R.duration(),
                    be = ze.end - ze.start;
                  ((P = ze.start + (be * P) / U),
                    (L = ze.start + (be * L) / U));
                }
                (u.refresh(
                  !1,
                  !1,
                  {
                    start: hr(P, A && !!u._startClamp),
                    end: hr(L, A && !!u._endClamp),
                  },
                  k,
                ),
                  u.update());
              }),
              (u.adjustPinSpacing = function (P) {
                if (we && P) {
                  var L = we.indexOf(b.d) + 1;
                  ((we[L] = parseFloat(we[L]) + P + Ne),
                    (we[1] = parseFloat(we[1]) + P + Ne),
                    Jn(we));
                }
              }),
              (u.disable = function (P, L) {
                if (
                  u.enabled &&
                  (P !== !1 && u.revert(!0, !0),
                  (u.enabled = u.isActive = !1),
                  L || (K && K.pause()),
                  (jt = 0),
                  qe && (qe.uncache = 1),
                  ke && Ge(d, "refreshInit", ke),
                  Xt &&
                    (Xt.pause(), dt.tween && dt.tween.kill() && (dt.tween = 0)),
                  !Le)
                ) {
                  for (var A = G.length; A--; )
                    if (G[A].scroller === N && G[A] !== u) return;
                  (Ge(N, "resize", Ei), Le || Ge(N, "scroll", qn));
                }
              }),
              (u.kill = function (P, L) {
                (u.disable(P, L), K && !L && K.kill(), O && delete Bo[O]);
                var A = G.indexOf(u);
                (A >= 0 && G.splice(A, 1),
                  A === ot && Yi > 0 && ot--,
                  (A = 0),
                  G.forEach(function (k) {
                    return k.scroller === u.scroller && (A = 1);
                  }),
                  A || at || (u.scroll.rec = 0),
                  n &&
                    ((n.scrollTrigger = null),
                    P && n.revert({ kill: !1 }),
                    L || n.kill()),
                  Pt &&
                    [Pt, wt, m, Dt].forEach(function (k) {
                      return k.parentNode && k.parentNode.removeChild(k);
                    }),
                  bi === u && (bi = 0),
                  h &&
                    (qe && (qe.uncache = 1),
                    (A = 0),
                    G.forEach(function (k) {
                      return k.pin === h && A++;
                    }),
                    A || (qe.spacer = 0)),
                  t.onKill && t.onKill(u));
              }),
              G.push(u),
              u.enable(!1, !1),
              un && un(u),
              n && n.add && !ye)
            ) {
              var Z = u.update;
              ((u.update = function () {
                ((u.update = Z), H.cache++, q || ve || u.refresh());
              }),
                v.delayedCall(0.01, u.update),
                (ye = 0.01),
                (q = ve = 0));
            } else u.refresh();
            h && qa();
          }),
          (d.register = function (t) {
            return (
              Xn ||
                ((v = t || Or()),
                Pr() && window.document && d.enable(),
                (Xn = _i)),
              Xn
            );
          }),
          (d.defaults = function (t) {
            if (t) for (var n in t) Wi[n] = t[n];
            return Wi;
          }),
          (d.disable = function (t, n) {
            ((_i = 0),
              G.forEach(function (_) {
                return _[n ? "kill" : "disable"](t);
              }),
              Ge(z, "wheel", qn),
              Ge(ae, "scroll", qn),
              clearInterval(Li),
              Ge(ae, "touchcancel", qt),
              Ge(ee, "touchstart", qt),
              Hi(Ge, ae, "pointerdown,touchstart,mousedown", Fr),
              Hi(Ge, ae, "pointerup,touchend,mouseup", Sr),
              Ri.kill(),
              Gi(Ge));
            for (var s = 0; s < H.length; s += 3)
              (zi(Ge, H[s], H[s + 1]), zi(Ge, H[s], H[s + 2]));
          }),
          (d.enable = function () {
            if (
              ((z = window),
              (ae = document),
              (Ct = ae.documentElement),
              (ee = ae.body),
              v &&
                ((ui = v.utils.toArray),
                (gi = v.utils.clamp),
                (To = v.core.context || qt),
                (Co = v.core.suppressOverwrites || qt),
                (wo = z.history.scrollRestoration || "auto"),
                (Wo = z.pageYOffset || 0),
                v.core.globals("ScrollTrigger", d),
                ee))
            ) {
              ((_i = 1),
                (Kn = document.createElement("div")),
                (Kn.style.height = "100vh"),
                (Kn.style.position = "absolute"),
                Rr(),
                Wa(),
                Oe.register(v),
                (d.isTouch = Oe.isTouch),
                (_n =
                  Oe.isTouch &&
                  /(iPad|iPhone|iPod|Mac)/g.test(navigator.userAgent)),
                (Ao = Oe.isTouch === 1),
                $e(z, "wheel", qn),
                (bo = [z, ae, Ct, ee]),
                v.matchMedia
                  ? ((d.matchMedia = function (D) {
                      var Q = v.matchMedia(),
                        j;
                      for (j in D) Q.add(j, D[j]);
                      return Q;
                    }),
                    v.addEventListener("matchMediaInit", function () {
                      return zo();
                    }),
                    v.addEventListener("matchMediaRevert", function () {
                      return Dr();
                    }),
                    v.addEventListener("matchMedia", function () {
                      (Rn(0, 1), Mn("matchMedia"));
                    }),
                    v.matchMedia().add("(orientation: portrait)", function () {
                      return (Ho(), Ho);
                    }))
                  : console.warn("Requires GSAP 3.11.0 or later"),
                Ho(),
                $e(ae, "scroll", qn));
              var t = ee.hasAttribute("style"),
                n = ee.style,
                s = n.borderTopStyle,
                _ = v.core.Animation.prototype,
                g,
                O;
              for (
                _.revert ||
                  Object.defineProperty(_, "revert", {
                    value: function () {
                      return this.time(-0.01, !0);
                    },
                  }),
                  n.borderTopStyle = "solid",
                  g = on(ee),
                  Me.m = Math.round(g.top + Me.sc()) || 0,
                  it.m = Math.round(g.left + it.sc()) || 0,
                  s
                    ? (n.borderTopStyle = s)
                    : n.removeProperty("border-top-style"),
                  t ||
                    (ee.setAttribute("style", ""), ee.removeAttribute("style")),
                  Li = setInterval(Tr, 250),
                  v.delayedCall(0.5, function () {
                    return ($i = 0);
                  }),
                  $e(ae, "touchcancel", qt),
                  $e(ee, "touchstart", qt),
                  Hi($e, ae, "pointerdown,touchstart,mousedown", Fr),
                  Hi($e, ae, "pointerup,touchend,mouseup", Sr),
                  xo = v.utils.checkPrefix("transform"),
                  Ui.push(xo),
                  Xn = Ye(),
                  Ri = v.delayedCall(0.2, Rn).pause(),
                  jn = [
                    ae,
                    "visibilitychange",
                    function () {
                      var D = z.innerWidth,
                        Q = z.innerHeight;
                      ae.hidden
                        ? ((pr = D), (dr = Q))
                        : (pr !== D || dr !== Q) && Ei();
                    },
                    ae,
                    "DOMContentLoaded",
                    Rn,
                    z,
                    "load",
                    Rn,
                    z,
                    "resize",
                    Ei,
                  ],
                  Gi($e),
                  G.forEach(function (D) {
                    return D.enable(0, 1);
                  }),
                  O = 0;
                O < H.length;
                O += 3
              )
                (zi(Ge, H[O], H[O + 1]), zi(Ge, H[O], H[O + 2]));
            }
          }),
          (d.config = function (t) {
            "limitCallbacks" in t && (No = !!t.limitCallbacks);
            var n = t.syncInterval;
            ((n && clearInterval(Li)) || ((Li = n) && setInterval(Tr, n)),
              "ignoreMobileResize" in t &&
                (Ao = d.isTouch === 1 && t.ignoreMobileResize),
              "autoRefreshEvents" in t &&
                (Gi(Ge) || Gi($e, t.autoRefreshEvents || "none"),
                (gr = (t.autoRefreshEvents + "").indexOf("resize") === -1)));
          }),
          (d.scrollerProxy = function (t, n) {
            var s = ht(t),
              _ = H.indexOf(s),
              g = Cn(s);
            (~_ && H.splice(_, g ? 6 : 2),
              n && (g ? Ut.unshift(z, n, ee, n, Ct, n) : Ut.unshift(s, n)));
          }),
          (d.clearMatchMedia = function (t) {
            G.forEach(function (n) {
              return n._ctx && n._ctx.query === t && n._ctx.kill(!0, !0);
            });
          }),
          (d.isInViewport = function (t, n, s) {
            var _ = (At(t) ? ht(t) : t).getBoundingClientRect(),
              g = _[s ? Tn : wn] * n || 0;
            return s
              ? _.right - g > 0 && _.left + g < z.innerWidth
              : _.bottom - g > 0 && _.top + g < z.innerHeight;
          }),
          (d.positionInViewport = function (t, n, s) {
            At(t) && (t = ht(t));
            var _ = t.getBoundingClientRect(),
              g = _[s ? Tn : wn],
              O =
                n == null
                  ? g / 2
                  : n in Vi
                    ? Vi[n] * g
                    : ~n.indexOf("%")
                      ? (parseFloat(n) * g) / 100
                      : parseFloat(n) || 0;
            return s
              ? (_.left + O) / z.innerWidth
              : (_.top + O) / z.innerHeight;
          }),
          (d.killAll = function (t) {
            if (
              (G.slice(0).forEach(function (s) {
                return s.vars.id !== "ScrollSmoother" && s.kill();
              }),
              t !== !0)
            ) {
              var n = Dn.killAll || [];
              ((Dn = {}),
                n.forEach(function (s) {
                  return s();
                }));
            }
          }),
          d
        );
      })();
    ((W.version = "3.12.7"),
      (W.saveStyles = function (d) {
        return d
          ? ui(d).forEach(function (e) {
              if (e && e.style) {
                var o = Tt.indexOf(e);
                (o >= 0 && Tt.splice(o, 5),
                  Tt.push(
                    e,
                    e.style.cssText,
                    e.getBBox && e.getAttribute("transform"),
                    v.core.getCache(e),
                    To(),
                  ));
              }
            })
          : Tt;
      }),
      (W.revert = function (d, e) {
        return zo(!d, e);
      }),
      (W.create = function (d, e) {
        return new W(d, e);
      }),
      (W.refresh = function (d) {
        return d ? Ei(!0) : (Xn || W.register()) && Rn(!0);
      }),
      (W.update = function (d) {
        return ++H.cache && rn(d === !0 ? 2 : 0);
      }),
      (W.clearScrollMemory = Mr),
      (W.maxScroll = function (d, e) {
        return Jt(d, e ? it : Me);
      }),
      (W.getScrollFunc = function (d, e) {
        return mn(ht(d), e ? it : Me);
      }),
      (W.getById = function (d) {
        return Bo[d];
      }),
      (W.getAll = function () {
        return G.filter(function (d) {
          return d.vars.id !== "ScrollSmoother";
        });
      }),
      (W.isScrolling = function () {
        return !!Lt;
      }),
      (W.snapDirectional = Go),
      (W.addEventListener = function (d, e) {
        var o = Dn[d] || (Dn[d] = []);
        ~o.indexOf(e) || o.push(e);
      }),
      (W.removeEventListener = function (d, e) {
        var o = Dn[d],
          t = o && o.indexOf(e);
        t >= 0 && o.splice(t, 1);
      }),
      (W.batch = function (d, e) {
        var o = [],
          t = {},
          n = e.interval || 0.016,
          s = e.batchMax || 1e9,
          _ = function (D, Q) {
            var j = [],
              C = [],
              h = v
                .delayedCall(n, function () {
                  (Q(j, C), (j = []), (C = []));
                })
                .pause();
            return function (T) {
              (j.length || h.restart(!0),
                j.push(T.trigger),
                C.push(T),
                s <= j.length && h.progress(1));
            };
          },
          g;
        for (g in e)
          t[g] =
            g.substr(0, 2) === "on" && Ue(e[g]) && g !== "onRefreshInit"
              ? _(g, e[g])
              : e[g];
        return (
          Ue(s) &&
            ((s = s()),
            $e(W, "refresh", function () {
              return (s = e.batchMax());
            })),
          ui(d).forEach(function (O) {
            var D = {};
            for (g in t) D[g] = t[g];
            ((D.trigger = O), o.push(W.create(D)));
          }),
          o
        );
      }));
    var Br = function (e, o, t, n) {
        return (
          o > n ? e(n) : o < 0 && e(0),
          t > n ? (n - o) / (t - o) : t < 0 ? o / (o - t) : 1
        );
      },
      jo = function d(e, o) {
        (o === !0
          ? e.style.removeProperty("touch-action")
          : (e.style.touchAction =
              o === !0
                ? "auto"
                : o
                  ? "pan-" + o + (Oe.isTouch ? " pinch-zoom" : "")
                  : "none"),
          e === Ct && d(ee, o));
      },
      Zi = { auto: 1, scroll: 1 },
      tl = function (e) {
        var o = e.event,
          t = e.target,
          n = e.axis,
          s = (o.changedTouches ? o.changedTouches[0] : o).target,
          _ = s._gsap || v.core.getCache(s),
          g = Ye(),
          O;
        if (!_._isScrollT || g - _._isScrollT > 2e3) {
          for (
            ;
            s &&
            s !== ee &&
            ((s.scrollHeight <= s.clientHeight &&
              s.scrollWidth <= s.clientWidth) ||
              !(Zi[(O = It(s)).overflowY] || Zi[O.overflowX]));
          )
            s = s.parentNode;
          ((_._isScroll =
            s &&
            s !== t &&
            !Cn(s) &&
            (Zi[(O = It(s)).overflowY] || Zi[O.overflowX])),
            (_._isScrollT = g));
        }
        (_._isScroll || n === "x") &&
          (o.stopPropagation(), (o._gsapAllow = !0));
      },
      Hr = function (e, o, t, n) {
        return Oe.create({
          target: e,
          capture: !0,
          debounce: !1,
          lockAxis: !0,
          type: o,
          onWheel: (n = n && tl),
          onPress: n,
          onDrag: n,
          onScroll: n,
          onEnable: function () {
            return t && $e(ae, Oe.eventTypes[0], Wr, !1, !0);
          },
          onDisable: function () {
            return Ge(ae, Oe.eventTypes[0], Wr, !0);
          },
        });
      },
      nl = /(input|label|select|textarea)/i,
      zr,
      Wr = function (e) {
        var o = nl.test(e.target.tagName);
        (o || zr) && ((e._gsapAllow = !0), (zr = o));
      },
      il = function (e) {
        (An(e) || (e = {}),
          (e.preventDefault = e.isNormalizer = e.allowClicks = !0),
          e.type || (e.type = "wheel,touch"),
          (e.debounce = !!e.debounce),
          (e.id = e.id || "normalizer"));
        var o = e,
          t = o.normalizeScrollX,
          n = o.momentum,
          s = o.allowNestedScroll,
          _ = o.onRelease,
          g,
          O,
          D = ht(e.target) || Ct,
          Q = v.core.globals().ScrollSmoother,
          j = Q && Q.get(),
          C =
            _n &&
            ((e.content && ht(e.content)) ||
              (j && e.content !== !1 && !j.smooth() && j.content())),
          h = mn(D, Me),
          T = mn(D, it),
          Fe = 1,
          le =
            (Oe.isTouch && z.visualViewport
              ? z.visualViewport.scale * z.visualViewport.width
              : z.outerWidth) / z.innerWidth,
          Re = 0,
          fe = Ue(n)
            ? function () {
                return n(g);
              }
            : function () {
                return n || 2.8;
              },
          Ve,
          $,
          $t = Hr(D, e.type, !0, s),
          ge = function () {
            return ($ = !1);
          },
          R = qt,
          lt = qt,
          Ft = function () {
            ((O = Jt(D, Me)),
              (lt = gi(_n ? 1 : 0, O)),
              t && (R = gi(0, Jt(D, it))),
              (Ve = Nn));
          },
          b = function () {
            ((C._gsap.y = fi(parseFloat(C._gsap.y) + h.offset) + "px"),
              (C.style.transform =
                "matrix3d(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, " +
                parseFloat(C._gsap.y) +
                ", 0, 1)"),
              (h.offset = h.cacheID = 0));
          },
          Be = function () {
            if ($) {
              requestAnimationFrame(ge);
              var Ce = fi(g.deltaY / 2),
                Ae = lt(h.v - Ce);
              if (C && Ae !== h.v + h.offset) {
                h.offset = Ae - h.v;
                var u = fi((parseFloat(C && C._gsap.y) || 0) - h.offset);
                ((C.style.transform =
                  "matrix3d(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, " +
                  u +
                  ", 0, 1)"),
                  (C._gsap.y = u + "px"),
                  (h.cacheID = H.cache),
                  rn());
              }
              return !0;
            }
            (h.offset && b(), ($ = !0));
          },
          N,
          an,
          Le,
          st,
          ct = function () {
            (Ft(),
              N.isActive() &&
                N.vars.scrollY > O &&
                (h() > O ? N.progress(1) && h(O) : N.resetTo("scrollY", O)));
          };
        return (
          C && v.set(C, { y: "+=0" }),
          (e.ignoreCheck = function (oe) {
            return (
              (_n && oe.type === "touchmove" && Be()) ||
              (Fe > 1.05 && oe.type !== "touchstart") ||
              g.isGesturing ||
              (oe.touches && oe.touches.length > 1)
            );
          }),
          (e.onPress = function () {
            $ = !1;
            var oe = Fe;
            ((Fe = fi(
              ((z.visualViewport && z.visualViewport.scale) || 1) / le,
            )),
              N.pause(),
              oe !== Fe && jo(D, Fe > 1.01 ? !0 : t ? !1 : "x"),
              (an = T()),
              (Le = h()),
              Ft(),
              (Ve = Nn));
          }),
          (e.onRelease = e.onGestureStart =
            function (oe, Ce) {
              if ((h.offset && b(), !Ce)) st.restart(!0);
              else {
                H.cache++;
                var Ae = fe(),
                  u,
                  ke;
                (t &&
                  ((u = T()),
                  (ke = u + (Ae * 0.05 * -oe.velocityX) / 0.227),
                  (Ae *= Br(T, u, ke, Jt(D, it))),
                  (N.vars.scrollX = R(ke))),
                  (u = h()),
                  (ke = u + (Ae * 0.05 * -oe.velocityY) / 0.227),
                  (Ae *= Br(h, u, ke, Jt(D, Me))),
                  (N.vars.scrollY = lt(ke)),
                  N.invalidate().duration(Ae).play(0.01),
                  ((_n && N.vars.scrollY >= O) || u >= O - 1) &&
                    v.to({}, { onUpdate: ct, duration: Ae }));
              }
              _ && _(oe);
            }),
          (e.onWheel = function () {
            (N._ts && N.pause(), Ye() - Re > 1e3 && ((Ve = 0), (Re = Ye())));
          }),
          (e.onChange = function (oe, Ce, Ae, u, ke) {
            if (
              (Nn !== Ve && Ft(),
              Ce &&
                t &&
                T(R(u[2] === Ce ? an + (oe.startX - oe.x) : T() + Ce - u[1])),
              Ae)
            ) {
              h.offset && b();
              var Ln = ke[2] === Ae,
                fn = Ln ? Le + oe.startY - oe.y : h() + Ae - ke[1],
                Gt = lt(fn);
              (Ln && fn !== Gt && (Le += Gt - fn), h(Gt));
            }
            (Ae || Ce) && rn();
          }),
          (e.onEnable = function () {
            (jo(D, t ? !1 : "x"),
              W.addEventListener("refresh", ct),
              $e(z, "resize", ct),
              h.smooth &&
                ((h.target.style.scrollBehavior = "auto"),
                (h.smooth = T.smooth = !1)),
              $t.enable());
          }),
          (e.onDisable = function () {
            (jo(D, !0),
              Ge(z, "resize", ct),
              W.removeEventListener("refresh", ct),
              $t.kill());
          }),
          (e.lockAxis = e.lockAxis !== !1),
          (g = new Oe(e)),
          (g.iOS = _n),
          _n && !h() && h(1),
          _n && v.ticker.add(qt),
          (st = g._dc),
          (N = v.to(g, {
            ease: "power4",
            paused: !0,
            inherit: !1,
            scrollX: t ? "+=0.1" : "+=0",
            scrollY: "+=0.1",
            modifiers: {
              scrollY: $r(h, h(), function () {
                return N.pause();
              }),
            },
            onUpdate: rn,
            onComplete: st.vars.onComplete,
          })),
          g
        );
      };
    ((W.sort = function (d) {
      if (Ue(d)) return G.sort(d);
      var e = z.pageYOffset || 0;
      return (
        W.getAll().forEach(function (o) {
          return (o._sortY = o.trigger
            ? e + o.trigger.getBoundingClientRect().top
            : o.start + z.innerHeight);
        }),
        G.sort(
          d ||
            function (o, t) {
              return (
                (o.vars.refreshPriority || 0) * -1e6 +
                (o.vars.containerAnimation ? 1e6 : o._sortY) -
                ((t.vars.containerAnimation ? 1e6 : t._sortY) +
                  (t.vars.refreshPriority || 0) * -1e6)
              );
            },
        )
      );
    }),
      (W.observe = function (d) {
        return new Oe(d);
      }),
      (W.normalizeScroll = function (d) {
        if (typeof d > "u") return rt;
        if (d === !0 && rt) return rt.enable();
        if (d === !1) {
          (rt && rt.kill(), (rt = d));
          return;
        }
        var e = d instanceof Oe ? d : il(d);
        return (
          rt && rt.target === e.target && rt.kill(),
          Cn(e.target) && (rt = e),
          e
        );
      }),
      (W.core = {
        _getVelocityProp: Eo,
        _inputObserver: Hr,
        _scrollers: H,
        _proxies: Ut,
        bridge: {
          ss: function () {
            (Lt || Mn("scrollStart"), (Lt = Ye()));
          },
          ref: function () {
            return Ke;
          },
        },
      }),
      Or() && v.registerPlugin(W),
      (c.ScrollTrigger = W),
      (c.default = W),
      typeof window > "u" || window !== c
        ? Object.defineProperty(c, "__esModule", { value: !0 })
        : delete window.default);
  });
});
var ya = (() => {
  class c {
    title = "landing";
    static ɵfac = function (a) {
      return new (a || c)();
    };
    static ɵcmp = J({
      type: c,
      selectors: [["pool-land-root"]],
      decls: 2,
      vars: 0,
      consts: [["position", "bottom-right", "richColors", ""]],
      template: function (a, r) {
        a & 1 && f(0, "router-outlet")(1, "ngx-sonner-toaster", 0);
      },
      dependencies: [ga, ca, ma],
      encapsulation: 2,
    });
  }
  return c;
})();
var Po = Xr(nr());
var ho = (() => {
  class c {
    static ɵfac = function (a) {
      return new (a || c)();
    };
    static ɵcmp = J({
      type: c,
      selectors: [["pool-land-payment-methods"]],
      decls: 8,
      vars: 0,
      consts: () => {
        let i;
        return (
          (i =
            "" +
            "\uFFFD#4\uFFFD\uFFFD/#4\uFFFD" +
            " Dibayarkan langsung ke platform POOOL Anda"),
          [
            i,
            [1, "flex", "flex-col", "gap-2"],
            [1, "inline-flex", "font-medium"],
            [
              "src",
              "svg/investment-amount-page-block/wallet.svg",
              "alt",
              "Wallet",
              1,
              "pr-2",
            ],
            [1, "flex", "gap-4"],
            ["src", "svg/investment-amount-page-block/visa.svg", "alt", "Visa"],
            [
              "src",
              "svg/investment-amount-page-block/master_card.svg",
              "alt",
              "Master Card",
            ],
          ]
        );
      },
      template: function (a, r) {
        a & 1 &&
          (l(0, "div", 1)(1, "p")(2, "span", 2),
          V(3, 0),
          f(4, "img", 3),
          X(),
          p()(),
          l(5, "div", 4),
          f(6, "img", 5)(7, "img", 6),
          p()());
      },
      dependencies: [ie],
      encapsulation: 2,
      changeDetection: 0,
    });
  }
  return c;
})();
function Fo() {
  let c = window.navigator.userAgent;
  return (
    c.indexOf("Safari") != -1 &&
    c.indexOf("Chrome") == -1 &&
    c.indexOf("Version/") != -1
  );
}
function So() {
  let c = window.navigator.userAgent;
  return /iP(ad|hone|od)/.test(c) && Fo() && !/CriOS/.test(c);
}
var rl = ["scrollContainer"],
  al = ["trigger"],
  ll = ["video"];
function sl(c, E) {
  c & 1 && f(0, "pool-land-payment-methods");
}
function cl(c, E) {
  if (
    (c & 1 &&
      (l(0, "div", 12)(1, "h4", 13),
      y(2),
      p(),
      l(3, "article", 14),
      y(4),
      p(),
      Nt(5, sl, 1, 0, "pool-land-payment-methods"),
      p()),
    c & 2)
  ) {
    let i = E.$implicit,
      a = E.$index;
    (F(2),
      ne(" ", i.title, " "),
      F(2),
      ne(" ", i.description, " "),
      F(),
      tn(a === 2 ? 5 : -1));
  }
}
var ba = (() => {
  class c {
    scrollContainer;
    trigger;
    video;
    platformId = De(oo);
    ngZone = De(io);
    scrollTriggerInstance = null;
    resizeObserver = null;
    initialTimeoutId = null;
    contents = [
      {
        title:
          "Kami yang menangani pekerjaannya, Anda yang mendapatkan keuntungannya",
        description:
          "Tim ahli kami memilih villa dan resor terbaik yang ditawarkan Bali - imbal hasil tinggi, tanpa kerumitan untuk Anda. Kami mengurus legalitas, perbankan, manajemen dan pajak. Bagian Anda? Hasilkan dan nikmati.",
      },
      {
        title: "Kepemilikan langsung, cepat, transparan, dan sepenuhnya aman",
        description: "Ketika Anda berinvestasi, Anda langsung memiliki saham.",
      },
      {
        title: "Pembayaran Bulanan, Keluar dengan Fleksibel",
        description:
          "Earn high rental income every month, no hassle with tax, management company  legal things etc.! Want to cash out? Sell your share on our marketplace  and extra benefit from property appreciation upon sale.",
      },
    ];
    ngAfterViewInit() {
      if (!(!uo(this.platformId) || So())) {
        try {
          (mo.registerPlugin(Po.default),
            (this.initialTimeoutId = window.setTimeout(() => {
              (this.initScrollTrigger(),
                window.addEventListener("resize", () => {
                  this.ngZone.run(() => {
                    this.refreshScrollTrigger();
                  });
                }));
            }, 500)));
        } catch (i) {
          console.error("Error in ngAfterViewInit:", i);
        }
        ((this.video.nativeElement.muted = !0),
          this.video.nativeElement.play());
      }
    }
    ngOnDestroy() {
      try {
        (this.scrollTriggerInstance &&
          (this.scrollTriggerInstance.kill(),
          (this.scrollTriggerInstance = null)),
          window.removeEventListener("resize", () => {
            this.refreshScrollTrigger();
          }),
          this.resizeObserver &&
            (this.resizeObserver.disconnect(), (this.resizeObserver = null)),
          this.initialTimeoutId !== null &&
            window.clearTimeout(this.initialTimeoutId));
      } catch (i) {
        console.error("Error in ngOnDestroy:", i);
      }
    }
    initScrollTrigger() {
      try {
        if (!this.scrollContainer || !this.trigger) return;
        let i = this.scrollContainer.nativeElement,
          a = this.trigger.nativeElement;
        this.scrollTriggerInstance && this.scrollTriggerInstance.kill();
        let r = i.scrollHeight - i.clientHeight;
        if (r <= 0) return;
        this.scrollTriggerInstance = Po.default.create({
          trigger: a,
          start: "top top",
          end: () => `+=${r}`,
          scrub: !0,
          pin: !0,
          pinSpacing: !0,
          anticipatePin: 1,
          onUpdate: (S) => {
            i && i.scrollTop !== void 0 && (i.scrollTop = S.progress * r);
          },
        });
      } catch (i) {
        console.error("Error initializing ScrollTrigger:", i);
      }
    }
    refreshScrollTrigger() {
      try {
        (this.scrollTriggerInstance &&
          (this.scrollTriggerInstance.kill(),
          (this.scrollTriggerInstance = null)),
          Po.default.refresh(),
          this.initScrollTrigger());
      } catch (i) {
        console.error("Error refreshing ScrollTrigger:", i);
      }
    }
    static ɵfac = function (a) {
      return new (a || c)();
    };
    static ɵcmp = J({
      type: c,
      selectors: [["pool-land-closer-look"]],
      viewQuery: function (a, r) {
        if ((a & 1 && (gt(rl, 5), gt(al, 5), gt(ll, 5)), a & 2)) {
          let S;
          (mt((S = _t())) && (r.scrollContainer = S.first),
            mt((S = _t())) && (r.trigger = S.first),
            mt((S = _t())) && (r.video = S.first));
        }
      },
      decls: 15,
      vars: 0,
      consts: () => {
        let i;
        return (
          (i =
            "" +
            "\uFFFD#4\uFFFD" +
            "Mari kita lihat lebih dekat " +
            "[\uFFFD/#4\uFFFD|\uFFFD/#5\uFFFD]" +
            "" +
            "\uFFFD#5\uFFFD" +
            "Cara kerjanya " +
            "[\uFFFD/#4\uFFFD|\uFFFD/#5\uFFFD]" +
            ""),
          (i = Et(i)),
          [
            ["trigger", ""],
            ["video", ""],
            ["scrollContainer", ""],
            i,
            [
              1,
              "max-block-width",
              "grid",
              "w-full",
              "grid-rows-[max-content_max-content]",
              "items-center",
              "justify-center",
              "gap-8",
              "pb-20",
              "pt-14",
              "lg:pb-28",
            ],
            [
              "bubbleAnimation",
              "",
              1,
              "mx-auto",
              "flex",
              "max-w-[80%]",
              "flex-col",
              "items-center",
              "justify-center",
              "text-center",
            ],
            [
              1,
              "text-3xl",
              "font-extrabold",
              "uppercase",
              "tracking-tighter",
              "lg:text-5xl",
              "xl:text-[56px]",
              "xl:leading-none",
            ],
            [
              1,
              "text-3xl",
              "font-extrabold",
              "uppercase",
              "tracking-tighter",
              "text-[#2B32F9]",
              "lg:text-5xl",
              "xl:text-6xl",
            ],
            [
              1,
              "flex",
              "h-full",
              "flex-col",
              "items-center",
              "gap-12",
              "lg:grid",
              "lg:grid-cols-2",
              "lg:place-items-center",
              "lg:gap-8",
              "lg:py-6",
            ],
            [1, "flex", "h-full", "items-center", "justify-center"],
            [
              "autoplay",
              "",
              "loop",
              "",
              "muted",
              "",
              "playsinline",
              "",
              "width",
              "100%",
              "height",
              "100%",
              "src",
              "webm/webm_phone_2.webm",
              1,
              "top-0",
              "h-full",
              "max-h-[480px]",
              "w-full",
            ],
            [
              1,
              "no-scrollbar",
              "flex",
              "flex-col",
              "items-center",
              "gap-8",
              "px-4",
              "lg:h-[500px]",
              "lg:overflow-hidden",
              "xl:h-[380px]",
            ],
            [
              1,
              "flex",
              "flex-col",
              "gap-4",
              "p-4",
              "sm:p-8",
              "lg:gap-6",
              "xl:p-24",
              "xl:pr-28",
            ],
            [
              1,
              "text-2xl",
              "font-bold",
              "tracking-tighter",
              "sm:text-4xl",
              "xl:text-5xl",
            ],
            [1, "text-base", "text-[#5A616E]", "sm:text-lg"],
          ]
        );
      },
      template: function (a, r) {
        (a & 1 &&
          (l(0, "div", 4, 0)(2, "div", 5),
          V(3, 3),
          f(4, "span", 6)(5, "span", 7),
          X(),
          p(),
          l(6, "div", 8)(7, "div", 9)(8, "video", 10, 1),
          y(10, " Unable to play the video "),
          p()(),
          l(11, "div", 11, 2),
          vt(13, cl, 6, 3, "div", 12, yn),
          p()()()),
          a & 2 && (F(13), yt(r.contents)));
      },
      dependencies: [ie, ho, We],
      encapsulation: 2,
      changeDetection: 0,
    });
  }
  return c;
})();
var pl = (c, E) => ({ "lg:h-[720px]": c, "lg:h-[662px]": E }),
  dl = (c, E, i, a, r) => ({
    "h-[710px]": c,
    "h-auto min-h-[959px]": E,
    "h-auto min-h-[1000px]": i,
    "lg:h-[720px]": a,
    "lg:h-[662px]": r,
  }),
  ul = (c, E, i) => ({
    "bottom-4 h-auto": c,
    "lg:h-[394px]": E,
    "lg:h-[336px]": i,
  }),
  gl = (c, E, i, a) => ({
    "line-clamp-[11] overflow-hidden": c,
    "h-auto overflow-visible": E,
    "lg:line-clamp-none": !0,
    "lg:overflow-visible": i,
    "lg:overflow-y-auto": a,
  }),
  ml = (c, E, i, a, r) => ({
    "h-[563px]": c,
    "h-auto min-h-[878px]": E,
    "h-auto min-h-[1050px]": i,
    "lg:h-[720px]": a,
    "lg:h-[646px]": r,
  }),
  _l = (c, E) => ({ "lg:h-[610px]": c, "lg:h-[536px]": E }),
  fl = (c, E, i) => ({
    "bottom-4 h-auto": c,
    "lg:h-[494px]": E,
    "lg:h-[420px]": i,
  }),
  hl = (c, E, i, a) => ({
    "line-clamp-[13] overflow-hidden": c,
    "h-auto overflow-visible": E,
    "lg:line-clamp-none": !0,
    "lg:overflow-visible": i,
    "lg:overflow-y-auto": a,
  }),
  Fl = (c, E) => ({ "lg:h-[720px]": c, "lg:h-[646px]": E }),
  xa = (() => {
    class c {
      locale = De(ai);
      expandedCards = { ryan: !1, monique: !1 };
      get isIndonesian() {
        return this.locale.includes("id");
      }
      toggleExpand(i) {
        this.expandedCards[i] = !this.expandedCards[i];
      }
      static ɵfac = function (a) {
        return new (a || c)();
      };
      static ɵcmp = J({
        type: c,
        selectors: [["pool-land-core-investors"]],
        decls: 95,
        vars: 50,
        consts: () => {
          let i;
          ((i =
            " Investor & Penasihat " +
            "\uFFFD#25\uFFFD" +
            "di " +
            "[\uFFFD/#25\uFFFD|\uFFFD/#26\uFFFD]" +
            "" +
            "\uFFFD#26\uFFFD" +
            "POOOL" +
            "[\uFFFD/#25\uFFFD|\uFFFD/#26\uFFFD]" +
            ""),
            (i = Et(i)));
          let a;
          a = "Pendiri Ankr";
          let r;
          r = "Kepala Pertumbuhan di USD1";
          let S;
          S =
            " Ryan Fang adalah pengusaha Web3 berpengalaman dan ahli strategi pertumbuhan dengan rekam jejak mendalam di bidang infrastruktur blockchain, DeFi, dan keuangan terdigitalisasi. ";
          let x;
          x =
            " Dia adalah " +
            "\uFFFD#48\uFFFD" +
            "Co-Founder dari Ankr" +
            "\uFFFD/#48\uFFFD" +
            ", salah satu platform infrastruktur Web3 terkemuka yang mendukung aplikasi terdesentralisasi. ";
          let B;
          B =
            " Saat ini menjabat sebagai " +
            "\uFFFD#51\uFFFD" +
            "Kepala Pertumbuhan di USD1 / World Liberty Financial" +
            "\uFFFD/#51\uFFFD" +
            ", Ryan memimpin adopsi global salah satu stablecoin dengan pertumbuhan tercepat di dunia, didukung oleh keluarga Trump\u2014 USD1 \u2014 yang telah melampaui volume $2B dan memposisikan dirinya sebagai jalur finansial inti untuk aset dunia nyata. ";
          let te;
          ((te =
            " Di " +
            "[\uFFFD#54\uFFFD|\uFFFD#55\uFFFD]" +
            "POOOL" +
            "[\uFFFD/#54\uFFFD|\uFFFD/#55\uFFFD]" +
            ", Ryan berperan sebagai investor awal dan penasihat strategis, mendukung ekspansi kami di Indonesia dan Asia Tenggara serta berkontribusi pada kesesuaian produk-pasar dalam lapisan integrasi RWA dan stablecoin. Wawasannya tentang penetrasi pasar, navigasi regulasi, dan akuisisi pengguna skala besar memainkan peran kunci dalam membentuk strategi global " +
            "[\uFFFD#54\uFFFD|\uFFFD#55\uFFFD]" +
            "POOOL" +
            "[\uFFFD/#54\uFFFD|\uFFFD/#55\uFFFD]" +
            ". "),
            (te = Et(te)));
          let Pe;
          ((Pe =
            " Investor ventura dan penasihat strategis " +
            "\uFFFD#73\uFFFD" +
            "di " +
            "[\uFFFD/#73\uFFFD|\uFFFD/#74\uFFFD]" +
            "" +
            "\uFFFD#74\uFFFD" +
            "POOOL" +
            "[\uFFFD/#73\uFFFD|\uFFFD/#74\uFFFD]" +
            ""),
            (Pe = Et(Pe)));
          let et;
          et =
            " Monique Howeth adalah " +
            "\uFFFD#81\uFFFD" +
            "investor ventura dan penasihat strategis di POOOL" +
            "\uFFFD/#81\uFFFD" +
            ", di mana dia memiliki 5% saham ekuitas dan memainkan peran penting dalam membentuk strategi modal perusahaan, penyelarasan regulasi, dan peningkatan skala internasional. Dengan pengalaman lebih dari 25 tahun di industri real estat AS dan latar belakang yang kuat dalam bisnis yang didukung ventura, Monique membawa keahlian operasional dan disiplin investor yang tak tertandingi ke ruang rapat POOOL. ";
          let de;
          ((de =
            " Kariernya mencakup pengembangan real estat residensial dan komersial, pertumbuhan waralaba, dan manajemen dana. Sebagai " +
            "[\uFFFD#84\uFFFD|\uFFFD#85\uFFFD]" +
            "mantan eksekutif di Crunch Fitness" +
            "[\uFFFD/#84\uFFFD|\uFFFD/#85\uFFFD]" +
            ", dia memainkan peran penting dalam mengembangkan salah satu " +
            "[\uFFFD#84\uFFFD|\uFFFD#85\uFFFD]" +
            "merek kebugaran paling terkenal di Amerika" +
            "[\uFFFD/#84\uFFFD|\uFFFD/#85\uFFFD]" +
            ", mengawasi ekspansi multi-unit dan operasi waralaba. Pengalaman kepemimpinan langsung ini, dikombinasikan dengan keahlian modal venturanya, memberinya perspektif ganda yang langka: pengetahuan operasional berbasis aset yang mendalam dan pandangan strategis pasar modal. "),
            (de = Et(de)));
          let xt;
          return (
            (xt =
              " Di " +
              "[\uFFFD#88\uFFFD|\uFFFD#89\uFFFD]" +
              "POOOL" +
              "[\uFFFD/#88\uFFFD|\uFFFD/#89\uFFFD]" +
              ", Monique bertindak sebagai investor sekaligus penasihat senior\u2014membimbing masuknya platform ke pasar-pasar baru seperti Indonesia, memastikan kesesuaian regulasi, dan menyelaraskan model bisnis untuk adopsi institusional jangka panjang. Keterlibatannya menegaskan kepercayaan pada tesis " +
              "[\uFFFD#88\uFFFD|\uFFFD#89\uFFFD]" +
              "POOOL" +
              "[\uFFFD/#88\uFFFD|\uFFFD/#89\uFFFD]" +
              ": bahwa real estat fraksional yang didukung blockchain adalah masa depan investasi properti global. "),
            (xt = Et(xt)),
            [
              i,
              a,
              r,
              S,
              x,
              B,
              te,
              Pe,
              et,
              de,
              xt,
              [1, "relative", "bg-[#F8F9FB]", 2, "padding-top", "120px"],
              [1, "mx-auto", "max-w-[1280px]", "px-4"],
              [1, "mb-6", "text-center", "lg:mb-8"],
              [1, "flex", "flex-col", "items-center"],
              [
                "src",
                "/svg/poool-text-logo.svg",
                "alt",
                "POOOL",
                1,
                "mb-3",
                "h-[40px]",
                "w-[153px]",
                "lg:h-[54px]",
                "lg:w-[206px]",
              ],
              [
                1,
                "w-full",
                "uppercase",
                "text-black",
                "lg:text-[56px]",
                "lg:font-extrabold",
                2,
                "font-family",
                "'TT Norms Pro'",
                "font-weight",
                "800",
                "font-size",
                "36px",
                "line-height",
                "100%",
                "letter-spacing",
                "-0.04em",
                "text-align",
                "center",
                "lg",
                "font-size: 56px",
                "lg",
                "font-weight: 800",
                "lg",
                "letter-spacing: -0.02em",
              ],
              [
                1,
                "flex",
                "flex-col",
                "gap-6",
                "lg:grid",
                "lg:grid-cols-2",
                "lg:justify-center",
                "lg:gap-x-4",
                "lg:gap-y-8",
                2,
                "max-width",
                "1134px",
                "margin",
                "0 auto",
              ],
              [1, "flex", "flex-col", "lg:contents"],
              [
                1,
                "relative",
                "mx-auto",
                "mb-4",
                "h-[350px]",
                "w-full",
                "max-w-[343px]",
                "overflow-hidden",
                "rounded-xl",
                "bg-white",
                "shadow-lg",
                "lg:hidden",
              ],
              [
                "src",
                "/webp/team/ryan-fang.webp",
                "alt",
                "Ryan Fang",
                1,
                "h-full",
                "w-full",
                "object-cover",
                "grayscale",
                2,
                "transform",
                "scale(1.05)",
                "transform-origin",
                "center center",
              ],
              [
                1,
                "relative",
                "hidden",
                "w-[559px]",
                "overflow-hidden",
                "rounded-xl",
                "bg-white",
                "shadow-lg",
                "lg:block",
                3,
                "ngClass",
              ],
              [
                "src",
                "/webp/team/ryan-fang.webp",
                "alt",
                "Ryan Fang",
                1,
                "h-full",
                "w-full",
                "object-cover",
                "grayscale",
                2,
                "transform",
                "scale(1.1)",
                "transform-origin",
                "center center",
              ],
              [
                1,
                "relative",
                "mx-auto",
                "w-[343px]",
                "overflow-hidden",
                "rounded-2xl",
                "border",
                "border-[#E2E2E2]",
                "bg-[#F3F8FB]",
                "lg:w-[559px]",
                2,
                "background-image",
                "url('/svg/background-buy-shares-with-gradient-animation.svg')",
                "background-size",
                "cover",
                "background-position",
                "center",
                3,
                "ngClass",
              ],
              [
                1,
                "pointer-events-none",
                "absolute",
                "inset-0",
                2,
                "background-image",
                "url('/svg/background-pattern-grid.svg')",
                "background-size",
                "cover",
                "background-position",
                "center",
              ],
              [
                1,
                "absolute",
                "left-4",
                "top-8",
                "z-10",
                "flex",
                "h-[22px]",
                "w-[200px]",
                "items-baseline",
                "gap-2",
                "lg:left-[327px]",
                "lg:bg-transparent",
                "lg:p-0",
              ],
              [
                "src",
                "/svg/poool-text-logo.svg",
                "alt",
                "POOOL",
                1,
                "h-[22px]",
                "w-[84px]",
                "lg:h-[22px]",
                "lg:w-[84px]",
              ],
              [
                1,
                "text-base",
                "text-[#141417]",
                2,
                "font-family",
                "'TT Norms Pro'",
                "font-weight",
                "600",
                "line-height",
                "100%",
                "letter-spacing",
                "0",
              ],
              [
                1,
                "relative",
                "p-4",
                "pt-[78px]",
                "lg:absolute",
                "lg:left-8",
                "lg:right-8",
                "lg:top-[78px]",
                "lg:h-[160px]",
                "lg:w-[495px]",
                "lg:p-0",
              ],
              [
                1,
                "flex",
                "flex-col",
                "gap-3",
                2,
                "height",
                "60px",
                "margin-bottom",
                "12px",
              ],
              [
                1,
                "text-black",
                2,
                "font-family",
                "'TT Norms Pro'",
                "font-weight",
                "600",
                "font-size",
                "32px",
                "line-height",
                "100%",
                "letter-spacing",
                "-0.04em",
                "width",
                "142px",
                "height",
                "32px",
              ],
              [
                1,
                "text-[#80858F]",
                2,
                "font-family",
                "'TT Norms Pro'",
                "font-weight",
                "500",
                "font-size",
                "12px",
                "line-height",
                "130%",
                "letter-spacing",
                "-0.04em",
                "width",
                "198px",
                "height",
                "16px",
              ],
              [
                2,
                "font-family",
                "'TT Norms Pro'",
                "font-weight",
                "400",
                "font-size",
                "12px",
                "line-height",
                "130%",
                "letter-spacing",
                "-0.04em",
              ],
              [
                1,
                "text-[#2B32F9]",
                2,
                "font-family",
                "'TT Norms Pro'",
                "font-weight",
                "700",
                "font-size",
                "12px",
                "line-height",
                "130%",
                "letter-spacing",
                "-0.04em",
              ],
              [
                1,
                "flex",
                "h-[153px]",
                "w-[311px]",
                "flex-col",
                "items-start",
                "p-1",
                "lg:h-[88px]",
                "lg:w-full",
                2,
                "background",
                "rgba(255, 255, 255, 0.3)",
                "border",
                "1px solid #ffffff",
                "backdrop-filter",
                "blur(12px)",
                "border-radius",
                "12px",
                "gap",
                "10px",
              ],
              [
                1,
                "flex",
                "h-[145px]",
                "w-[303px]",
                "flex-col",
                "items-center",
                "justify-center",
                "rounded-xl",
                "bg-[#98FB96]",
                "p-2",
                "lg:h-full",
                "lg:w-full",
                "lg:flex-row",
                "lg:p-4",
                2,
                "gap",
                "10px",
              ],
              [
                1,
                "flex",
                "flex-col",
                "items-center",
                "justify-center",
                "gap-4",
                "lg:flex-row",
                "lg:gap-8",
              ],
              [1, "flex", "items-center", "gap-2"],
              [
                1,
                "flex",
                "h-12",
                "w-12",
                "items-center",
                "justify-center",
                "rounded-full",
                "bg-white",
                "p-0",
              ],
              [
                "src",
                "/png/ankr-logo.webp",
                "alt",
                "Ankr",
                1,
                "h-12",
                "w-12",
                "object-contain",
              ],
              [
                1,
                "text-xs",
                "font-normal",
                "text-[#141417]",
                2,
                "line-height",
                "120%",
              ],
              [
                1,
                "h-[1px]",
                "w-[287px]",
                "rounded-lg",
                "bg-[#2B32F9]",
                "lg:h-8",
                "lg:w-px",
              ],
              [
                "src",
                "/png/usd1-logo.webp",
                "alt",
                "USD1",
                1,
                "h-12",
                "w-12",
                "rounded-full",
                "object-contain",
              ],
              [
                1,
                "absolute",
                "left-4",
                "top-[319px]",
                "lg:left-8",
                "lg:top-[254px]",
              ],
              ["src", "/svg/quote-dots.svg", "alt", "", 1, "h-6", "w-[29px]"],
              [
                1,
                "absolute",
                "left-4",
                "top-[359px]",
                "w-[311px]",
                "lg:bottom-8",
                "lg:left-8",
                "lg:right-8",
                "lg:top-[294px]",
                "lg:h-[394px]",
                "lg:w-[495px]",
                3,
                "ngClass",
              ],
              [
                1,
                "text-[#141417]",
                "lg:h-full",
                "lg:overflow-visible",
                2,
                "font-family",
                "'TT Norms Pro'",
                "font-weight",
                "400",
                "font-size",
                "14px",
                "line-height",
                "150%",
                "letter-spacing",
                "0",
                3,
                "ngClass",
              ],
              [2, "margin-bottom", "20px"],
              [
                2,
                "font-family",
                "'TT Norms Pro'",
                "font-weight",
                "600",
                "font-size",
                "14px",
                "line-height",
                "150%",
                "letter-spacing",
                "0",
                "color",
                "#2b32f9",
              ],
              [
                2,
                "font-family",
                "'TT Norms Pro'",
                "font-weight",
                "700",
                "font-size",
                "14px",
                "line-height",
                "150%",
                "letter-spacing",
                "0",
                "color",
                "#2b32f9",
              ],
              [
                1,
                "mb-8",
                "mt-4",
                "flex",
                "h-[35px]",
                "w-[77px]",
                "items-center",
                "justify-center",
                "rounded-[50px]",
                "text-[#2B32F9]",
                "lg:hidden",
                2,
                "font-family",
                "'TT Norms Pro'",
                "font-weight",
                "600",
                "font-size",
                "16px",
                "line-height",
                "100%",
                "letter-spacing",
                "-0.04em",
                "text-align",
                "center",
                3,
                "click",
              ],
              [
                "src",
                "/png/monique-howeth-mobile.webp",
                "alt",
                "Monique Howeth",
                1,
                "h-full",
                "w-full",
                "object-cover",
                "object-center",
                "grayscale",
              ],
              [
                1,
                "relative",
                "flex",
                "flex-col",
                "items-start",
                "gap-4",
                "p-4",
                "pt-[78px]",
                "lg:absolute",
                "lg:left-8",
                "lg:top-[78px]",
                "lg:w-[495px]",
                "lg:p-0",
                3,
                "ngClass",
              ],
              [
                1,
                "flex",
                "flex-col",
                "items-start",
                "gap-3",
                2,
                "width",
                "495px",
                "height",
                "60px",
              ],
              [
                1,
                "text-black",
                2,
                "font-family",
                "'TT Norms Pro'",
                "font-weight",
                "600",
                "font-size",
                "32px",
                "line-height",
                "100%",
                "letter-spacing",
                "-0.04em",
                "width",
                "238px",
                "height",
                "32px",
              ],
              [
                1,
                "text-[#80858F]",
                2,
                "font-family",
                "'TT Norms Pro'",
                "font-weight",
                "400",
                "font-size",
                "12px",
                "line-height",
                "130%",
                "letter-spacing",
                "-0.04em",
                "width",
                "320px",
                "height",
                "16px",
              ],
              [
                1,
                "absolute",
                "left-4",
                "top-[154px]",
                "lg:relative",
                "lg:left-auto",
                "lg:top-auto",
                2,
                "width",
                "29px",
                "height",
                "24px",
              ],
              [
                "src",
                "/svg/quote-dots.svg",
                "alt",
                "",
                2,
                "width",
                "29px",
                "height",
                "24px",
              ],
              [
                1,
                "absolute",
                "left-4",
                "top-[194px]",
                "w-[311px]",
                "lg:relative",
                "lg:bottom-auto",
                "lg:left-auto",
                "lg:right-auto",
                "lg:top-auto",
                "lg:h-[494px]",
                "lg:w-[495px]",
                3,
                "ngClass",
              ],
              [
                1,
                "text-[#141417]",
                "lg:h-full",
                "lg:overflow-visible",
                2,
                "font-family",
                "'TT Norms Pro'",
                "font-weight",
                "400",
                "font-size",
                "14px",
                "line-height",
                "148%",
                "letter-spacing",
                "0",
                3,
                "ngClass",
              ],
              [
                1,
                "relative",
                "hidden",
                "w-full",
                "overflow-hidden",
                "rounded-xl",
                "bg-white",
                "shadow-lg",
                "lg:block",
                "lg:w-[559px]",
                3,
                "ngClass",
              ],
              [
                "src",
                "/webp/team/monique-howeth.webp",
                "alt",
                "Monique Howeth",
                1,
                "h-full",
                "w-full",
                "object-cover",
                "grayscale",
              ],
              [2, "height", "120px"],
            ]
          );
        },
        template: function (a, r) {
          (a & 1 &&
            (l(0, "section", 11)(1, "div", 12)(2, "div", 13)(3, "div", 14),
            f(4, "img", 15),
            l(5, "h2", 16),
            y(6, " Core Investors "),
            p()()(),
            l(7, "div", 17)(8, "div", 18)(9, "div", 19),
            f(10, "img", 20),
            p(),
            l(11, "div", 21),
            f(12, "img", 22),
            p(),
            l(13, "div", 23),
            f(14, "div", 24),
            l(15, "div", 25),
            f(16, "img", 26),
            l(17, "span", 27),
            y(18, "Core Investors"),
            p()(),
            l(19, "div", 28)(20, "div", 29)(21, "h3", 30),
            y(22, " Ryan Fang "),
            p(),
            l(23, "p", 31),
            V(24, 0),
            f(25, "span", 32)(26, "span", 33),
            X(),
            p()(),
            l(27, "div", 34)(28, "div", 35)(29, "div", 36)(30, "div", 37)(
              31,
              "div",
              38,
            ),
            f(32, "img", 39),
            p(),
            l(33, "span", 40),
            Y(34, 1),
            p()(),
            f(35, "div", 41),
            l(36, "div", 37),
            f(37, "img", 42),
            l(38, "span", 40),
            Y(39, 2),
            p()()()()()(),
            l(40, "div", 43),
            f(41, "img", 44),
            p(),
            l(42, "div", 45)(43, "div", 46)(44, "p", 47),
            Y(45, 3),
            p(),
            l(46, "p", 47),
            V(47, 4),
            f(48, "span", 48),
            X(),
            p(),
            l(49, "p", 47),
            V(50, 5),
            f(51, "span", 48),
            X(),
            p(),
            l(52, "p"),
            V(53, 6),
            f(54, "span", 49)(55, "span", 49),
            X(),
            p()(),
            l(56, "button", 50),
            pe("click", function () {
              return r.toggleExpand("ryan");
            }),
            y(57),
            p()()()(),
            l(58, "div", 18)(59, "div", 19),
            f(60, "img", 51),
            p(),
            l(61, "div", 23),
            f(62, "div", 24),
            l(63, "div", 25),
            f(64, "img", 26),
            l(65, "span", 27),
            y(66, "Core Investors"),
            p()(),
            l(67, "div", 52)(68, "div", 53)(69, "h3", 54),
            y(70, " Monique Howeth "),
            p(),
            l(71, "p", 55),
            V(72, 7),
            f(73, "span", 32)(74, "span", 33),
            X(),
            p()(),
            l(75, "div", 56),
            f(76, "img", 57),
            p(),
            l(77, "div", 58)(78, "div", 59)(79, "p", 47),
            V(80, 8),
            f(81, "span", 48),
            X(),
            p(),
            l(82, "p", 47),
            V(83, 9),
            f(84, "span", 48)(85, "span", 48),
            X(),
            p(),
            l(86, "p"),
            V(87, 10),
            f(88, "span", 49)(89, "span", 49),
            X(),
            p()(),
            l(90, "button", 50),
            pe("click", function () {
              return r.toggleExpand("monique");
            }),
            y(91),
            p()()()()(),
            l(92, "div", 60),
            f(93, "img", 61),
            p()()(),
            f(94, "div", 62),
            p()),
            a & 2 &&
              (F(11),
              M("ngClass", En(11, pl, r.isIndonesian, !r.isIndonesian)),
              F(2),
              M(
                "ngClass",
                er(
                  14,
                  dl,
                  !r.expandedCards.ryan,
                  r.expandedCards.ryan && !r.isIndonesian,
                  r.expandedCards.ryan && r.isIndonesian,
                  r.isIndonesian,
                  !r.isIndonesian,
                ),
              ),
              F(29),
              M(
                "ngClass",
                Qo(
                  20,
                  ul,
                  r.expandedCards.ryan,
                  r.isIndonesian,
                  !r.isIndonesian,
                ),
              ),
              F(),
              M(
                "ngClass",
                Zo(
                  24,
                  gl,
                  !r.expandedCards.ryan,
                  r.expandedCards.ryan,
                  r.isIndonesian,
                  !r.isIndonesian,
                ),
              ),
              F(14),
              ne(" ", r.expandedCards.ryan ? "Show Less" : "Show More", " "),
              F(4),
              M(
                "ngClass",
                er(
                  29,
                  ml,
                  !r.expandedCards.monique,
                  r.expandedCards.monique && !r.isIndonesian,
                  r.expandedCards.monique && r.isIndonesian,
                  r.isIndonesian,
                  !r.isIndonesian,
                ),
              ),
              F(6),
              M("ngClass", En(35, _l, r.isIndonesian, !r.isIndonesian)),
              F(10),
              M(
                "ngClass",
                Qo(
                  38,
                  fl,
                  r.expandedCards.monique,
                  r.isIndonesian,
                  !r.isIndonesian,
                ),
              ),
              F(),
              M(
                "ngClass",
                Zo(
                  42,
                  hl,
                  !r.expandedCards.monique,
                  r.expandedCards.monique,
                  r.isIndonesian,
                  !r.isIndonesian,
                ),
              ),
              F(13),
              ne(" ", r.expandedCards.monique ? "Show Less" : "Show More", " "),
              F(),
              M("ngClass", En(47, Fl, r.isIndonesian, !r.isIndonesian))));
        },
        dependencies: [ie, li],
        styles: [
          '[_nghost-%COMP%]{display:block}section[_ngcontent-%COMP%]{position:relative}section[_ngcontent-%COMP%]:before{content:"";position:absolute;width:764px;height:700px;left:39px;top:120px;background:#98fb960f;filter:blur(100px);pointer-events:none}section[_ngcontent-%COMP%]:after{content:"";position:absolute;width:818px;height:750px;right:39px;bottom:120px;background:#001dca0f;filter:blur(100px);transform:rotate(54.66deg);pointer-events:none}',
        ],
      });
    }
    return c;
  })();
var Sl = ["video"];
function Pl(c, E) {
  c & 1 && f(0, "source", 20);
}
function Ol(c, E) {
  if (c & 1) {
    let i = Ht();
    (l(0, "div", 24),
      pe("click", function () {
        se(i);
        let r = _e();
        return ce(r.handleVideoClick(!1));
      }),
      l(1, "div", 25),
      pe("click", function (r) {
        return (se(i), ce(r.stopPropagation()));
      }),
      l(2, "button", 26),
      pe("click", function () {
        se(i);
        let r = _e();
        return ce(r.handleVideoClick(!1));
      }),
      l(3, "span", 27),
      y(4, "\xD7"),
      p()(),
      l(5, "div", 28),
      f(6, "iframe", 29),
      bt(7, "safeUrl"),
      p()()());
  }
  if (c & 2) {
    let i = _e();
    (F(6), M("src", zn(7, 1, i.videoSrc), ro));
  }
}
var Ca = (() => {
  class c {
    locale = De(ai);
    navbarLinks;
    video;
    get isSafariOrSafariMobile() {
      return Fo() || /iP(ad|hone|od)/.test(window.navigator.userAgent);
    }
    get isIndonesian() {
      return this.locale.includes("id");
    }
    cd = De(bn);
    videoSrc =
      "https://www.youtube.com/embed/GTSeeou3Wg8?si=jfpfH1FtJ9qzX14D?autoplay=1";
    showVideo = !1;
    ngAfterViewInit() {
      ((this.video.nativeElement.muted = !0), this.video.nativeElement.play());
    }
    handleVideoClick(i) {
      ((this.showVideo = i), this.cd.detectChanges());
    }
    static ɵfac = function (a) {
      return new (a || c)();
    };
    static ɵcmp = J({
      type: c,
      selectors: [["pool-land-home-hero"]],
      viewQuery: function (a, r) {
        if ((a & 1 && gt(Sl, 5), a & 2)) {
          let S;
          mt((S = _t())) && (r.video = S.first);
        }
      },
      inputs: { navbarLinks: "navbarLinks" },
      decls: 28,
      vars: 5,
      consts: () => {
        let i;
        i = "Masuk";
        let a;
        a = "Mulai dari Rp. 150.000";
        let r;
        r = "Tonton video";
        let S;
        ((S =
          " " +
          "\uFFFD#12\uFFFD" +
          "miliki " +
          "\uFFFD#13\uFFFD" +
          "" +
          "\uFFFD/#13\uFFFD" +
          "" +
          "\uFFFD/#12\uFFFD" +
          "" +
          "[\uFFFD#10\uFFFD\uFFFD/#10\uFFFD|\uFFFD#11\uFFFD\uFFFD/#11\uFFFD|\uFFFD#14\uFFFD\uFFFD/#14\uFFFD]" +
          " sebagian dari bali " +
          "[\uFFFD#10\uFFFD\uFFFD/#10\uFFFD|\uFFFD#11\uFFFD\uFFFD/#11\uFFFD|\uFFFD#14\uFFFD\uFFFD/#14\uFFFD]" +
          " villa " +
          "[\uFFFD#10\uFFFD\uFFFD/#10\uFFFD|\uFFFD#11\uFFFD\uFFFD/#11\uFFFD|\uFFFD#14\uFFFD\uFFFD/#14\uFFFD]" +
          " & dapatkan penghasilan bulanan "),
          (S = Et(S)));
        let x;
        return (
          (x =
            "Ribuan investor memiliki saham di pasar Bali yang sedang berkembang"),
          [
            ["video", ""],
            S,
            x,
            [
              1,
              "grid",
              "min-h-screen",
              "w-screen",
              "grid-rows-[max-content_1fr]",
              "place-items-center",
              "bg-[url(/png/MobileHeroBg.webp)]",
              "bg-cover",
              "bg-clip-content",
              "bg-center",
              "bg-no-repeat",
              "lg:bg-[#0024D1]",
              "lg:bg-[url(/svg/bg-1.svg)]",
            ],
            [1, "w-full", 3, "navbarLinks"],
            [
              "ngProjectAs",
              "pool-hero-button",
              5,
              ["pool-hero-button"],
              1,
              "flex",
              "items-center",
              "gap-8",
            ],
            ["variant", "secondary"],
            [
              "href",
              "/auth/login",
              "target",
              "_blank",
              "rel",
              "noopener noreferrer",
            ],
            ["text", i, "variant", "primary-secondary-green"],
            [
              1,
              "flex",
              "h-full",
              "max-w-[1440px]",
              "flex-col-reverse",
              "items-center",
              "justify-center",
              "gap-8",
              "p-4",
              "pb-20",
              "lg:grid",
              "lg:h-max",
              "lg:grid-cols-[min-content_1fr]",
              "lg:place-items-center",
              "lg:items-center",
              "lg:gap-8",
              "lg:px-8",
              "xl:pl-16",
            ],
            [
              1,
              "flex",
              "w-fit",
              "flex-col",
              "items-center",
              "justify-center",
              "gap-4",
              "text-wrap",
              "text-center",
              "lg:w-min",
              "lg:items-start",
              "lg:justify-start",
              "lg:gap-6",
              "lg:pt-10",
              "lg:text-start",
            ],
            [
              1,
              "text-3xl",
              "font-extrabold",
              "uppercase",
              "tracking-tighter",
              "text-[#dbecff]",
              "lg:text-5xl",
              "xl:text-6xl",
            ],
            [
              1,
              "inline-flex",
              "flex-col",
              "items-center",
              "gap-1",
              "pb-1",
              "lg:flex-row",
              "lg:gap-4",
              "lg:pb-0",
            ],
            [
              "text",
              a,
              "variant",
              "secondary",
              1,
              "inline-flex",
              "tracking-normal",
            ],
            [
              1,
              "whitespace-normal",
              "text-wrap",
              "text-base",
              "text-[#dbecff]",
              "lg:text-2xl",
            ],
            [
              1,
              "grid",
              "w-full",
              "grid-cols-1",
              "grid-rows-1",
              "gap-2",
              "lg:w-max",
              "lg:grid-cols-2",
              "lg:gap-4",
            ],
            ["tabindex", "0", 3, "click", "keydown.enter"],
            ["text", r, "variant", "animated-border", 1, "w-full"],
            ["src", "/svg/play-circle.svg", "alt", "play-circle", 1, "h-5"],
            [
              "playsinline",
              "",
              "autoplay",
              "",
              "muted",
              "",
              1,
              "h-full",
              "w-full",
            ],
            ["src", "mp4/HEVC_Villa.mov", "type", 'video/mp4; codecs="hvc1"'],
            ["src", "/webm/webm_villa_no-bg.webm", "type", "video/webm"],
            ["src", "/webm/webm_villa.webm", "type", "video/webm"],
            [
              "class",
              "fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-80 p-4",
              3,
              "click",
              4,
              "ngIf",
            ],
            [
              1,
              "fixed",
              "inset-0",
              "z-50",
              "flex",
              "items-center",
              "justify-center",
              "bg-black",
              "bg-opacity-80",
              "p-4",
              3,
              "click",
            ],
            [
              1,
              "relative",
              "w-full",
              "max-w-4xl",
              "rounded-xl",
              "bg-black",
              "shadow-2xl",
              3,
              "click",
            ],
            [
              "aria-label",
              "Close video",
              1,
              "absolute",
              "-right-4",
              "-top-4",
              "flex",
              "h-10",
              "w-10",
              "items-center",
              "justify-center",
              "rounded-full",
              "bg-white",
              "text-black",
              "transition-all",
              "hover:bg-gray-200",
              3,
              "click",
            ],
            [1, "text-2xl"],
            [1, "aspect-video", "w-full", "overflow-hidden", "rounded-xl"],
            [
              "width",
              "100%",
              "height",
              "100%",
              "frameborder",
              "0",
              "allow",
              "accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture",
              "allowfullscreen",
              "",
              3,
              "src",
            ],
          ]
        );
      },
      template: function (a, r) {
        if (a & 1) {
          let S = Ht();
          (l(0, "div", 3)(1, "pool-land-header", 4)(2, "div", 5),
            f(3, "pool-land-language-selector", 6),
            l(4, "a", 7),
            f(5, "pool-land-button", 8),
            p()()(),
            l(6, "article", 9)(7, "div", 10)(8, "h1", 11),
            V(9, 1),
            f(10, "br")(11, "br"),
            l(12, "span", 12),
            f(13, "pool-land-button", 13),
            p(),
            f(14, "br"),
            X(),
            p(),
            l(15, "p", 14),
            Y(16, 2),
            p(),
            l(17, "div", 15)(18, "a", 16),
            pe("click", function () {
              return (se(S), ce(r.handleVideoClick(!0)));
            })("keydown.enter", function () {
              return (se(S), ce(r.handleVideoClick(!0)));
            }),
            l(19, "pool-land-button", 17),
            f(20, "img", 18),
            p()()()(),
            l(21, "div")(22, "video", 19, 0),
            Nt(24, Pl, 1, 0, "source", 20),
            f(25, "source", 21)(26, "source", 22),
            p()()()(),
            Nt(27, Ol, 8, 3, "div", 23));
        }
        a & 2 &&
          (F(),
          M("navbarLinks", r.navbarLinks),
          F(7),
          ao("text-nowrap", !r.isIndonesian),
          F(16),
          tn(r.isSafariOrSafariMobile ? 24 : -1),
          F(3),
          M("ngIf", r.showVideo));
      },
      dependencies: [ie, co, xn, Fa, go, _o],
      encapsulation: 2,
      changeDetection: 0,
    });
  }
  return c;
})();
var Oo = Xr(nr());
var vl = ["scrollContainer"],
  yl = ["trigger"],
  El = ["video"];
function bl(c, E) {
  c & 1 && f(0, "pool-land-payment-methods");
}
function xl(c, E) {
  if (
    (c & 1 &&
      (l(0, "div", 11)(1, "span", 14),
      y(2),
      p(),
      f(3, "p", 15),
      l(4, "p", 16),
      y(5),
      p(),
      Nt(6, bl, 1, 0, "pool-land-payment-methods"),
      p()),
    c & 2)
  ) {
    let i = E.$implicit,
      a = E.$index;
    (F(2),
      ne(" ", i.title, " "),
      F(),
      M("innerHTML", i.description, vn),
      F(2),
      ne(" ", i.content, " "),
      F(),
      tn(a === 2 ? 6 : -1));
  }
}
var Aa = (() => {
  class c {
    scrollContainer;
    trigger;
    video;
    platformId = De(oo);
    ngZone = De(io);
    scrollTriggerInstance = null;
    resizeObserver = null;
    initialTimeoutId = null;
    componentProps = [
      {
        title: "Daftar",
        description:
          "Buat akun Anda, lakukan KYC dan Anda siap untuk berinvestasi",
        content:
          "Daftar dalam waktu kurang dari 3 menit dan telusuri koleksi real estat dan bisnis kami, yang bersumber dari para ahli kami.",
      },
      {
        title: "Jelajahi Properti",
        description: "Jelajahi. Bandingkan. Pilih favorit Anda",
        content:
          "Gulir opsi yang dipilih sendiri, bandingkan detailnya, dan pilih yang paling sesuai dengan portofolio Anda.",
      },
      {
        title: "Berinvestasi Dalam Saham",
        description:
          'Mulai dengan <span class="text-primary-blue">150.000 Rp</span>. Miliki saham Anda dalam hitungan menit',
        content:
          "Lewati kerumitan, dan beli saham di penawaran favorit Anda, di mana pun Anda berada di dunia.",
      },
      {
        title: "Hasilkan Uang dengan Mudah dan Jual Cepat",
        description: "Keluar dengan cepat dan sederhana",
        content:
          "Nikmati pendapatan sewa bulanan, fleksibilitas untuk menjual di Marketplace kami kapan pun Anda mau, dan akses ke opsi exit strategy kami yang lain.",
      },
    ];
    ngAfterViewInit() {
      if (!(!uo(this.platformId) || So())) {
        try {
          (mo.registerPlugin(Oo.default),
            (this.initialTimeoutId = window.setTimeout(() => {
              (this.initScrollTrigger(),
                window.addEventListener("resize", () => {
                  this.ngZone.run(() => {
                    this.refreshScrollTrigger();
                  });
                }));
            }, 500)));
        } catch (i) {
          console.error("Error in ngAfterViewInit:", i);
        }
        ((this.video.nativeElement.muted = !0),
          this.video.nativeElement.play());
      }
    }
    ngOnDestroy() {
      try {
        (this.scrollTriggerInstance &&
          (this.scrollTriggerInstance.kill(),
          (this.scrollTriggerInstance = null)),
          window.removeEventListener("resize", () => {
            this.refreshScrollTrigger();
          }),
          this.resizeObserver &&
            (this.resizeObserver.disconnect(), (this.resizeObserver = null)),
          this.initialTimeoutId !== null &&
            window.clearTimeout(this.initialTimeoutId));
      } catch (i) {
        console.error("Error in ngOnDestroy:", i);
      }
    }
    initScrollTrigger() {
      try {
        if (!this.scrollContainer || !this.trigger) return;
        let i = this.scrollContainer.nativeElement,
          a = this.trigger.nativeElement;
        this.scrollTriggerInstance && this.scrollTriggerInstance.kill();
        let r = i.scrollHeight - i.clientHeight;
        if (r <= 0) return;
        this.scrollTriggerInstance = Oo.default.create({
          trigger: a,
          start: "top top",
          end: () => `+=${r}`,
          scrub: !0,
          pin: !0,
          pinSpacing: !0,
          anticipatePin: 1,
          onUpdate: (S) => {
            i && i.scrollTop !== void 0 && (i.scrollTop = S.progress * r);
          },
        });
      } catch (i) {
        console.error("Error initializing ScrollTrigger:", i);
      }
    }
    refreshScrollTrigger() {
      try {
        (this.scrollTriggerInstance &&
          (this.scrollTriggerInstance.kill(),
          (this.scrollTriggerInstance = null)),
          Oo.default.refresh(),
          this.initScrollTrigger());
      } catch (i) {
        console.error("Error refreshing ScrollTrigger:", i);
      }
    }
    static ɵfac = function (a) {
      return new (a || c)();
    };
    static ɵcmp = J({
      type: c,
      selectors: [["pool-land-investment-amount"]],
      viewQuery: function (a, r) {
        if ((a & 1 && (gt(vl, 5), gt(yl, 5), gt(El, 5)), a & 2)) {
          let S;
          (mt((S = _t())) && (r.scrollContainer = S.first),
            mt((S = _t())) && (r.trigger = S.first),
            mt((S = _t())) && (r.video = S.first));
        }
      },
      decls: 17,
      vars: 0,
      consts: () => {
        let i;
        i = "Bagaimana cara kerjanya?";
        let a;
        return (
          (a =
            " Dapatkan penghasilan bulanan" +
            "\uFFFD#7\uFFFD\uFFFD/#7\uFFFD" +
            " dari properti yang berkembang "),
          [
            ["trigger", ""],
            ["scrollContainer", ""],
            ["video", ""],
            i,
            a,
            [
              1,
              "max-block-width",
              "grid",
              "w-full",
              "grid-rows-[max-content_max-content]",
              "items-center",
              "justify-center",
              "gap-4",
              "sm:gap-8",
            ],
            [
              "bubbleAnimation",
              "",
              1,
              "flex",
              "flex-col",
              "items-center",
              "justify-center",
              "gap-4",
              "xl:gap-6",
            ],
            [1, "text-secondary-blue", "text-base", "font-bold", "xl:text-lg"],
            [
              1,
              "max-w-[80%]",
              "text-center",
              "text-3xl",
              "font-extrabold",
              "uppercase",
              "tracking-tighter",
              "lg:text-5xl",
              "xl:text-[56px]",
              "xl:leading-none",
            ],
            [
              1,
              "flex",
              "flex-col-reverse",
              "gap-4",
              "lg:grid",
              "lg:grid-cols-2",
              "lg:place-items-center",
              "lg:gap-8",
              "lg:py-6",
            ],
            [
              1,
              "no-scrollbar",
              "flex",
              "flex-col",
              "items-center",
              "gap-8",
              "px-4",
              "lg:h-[500px]",
              "lg:overflow-hidden",
              "xl:h-[380px]",
            ],
            [1, "scroll-container__item"],
            [1, "flex", "h-full", "items-center", "justify-center"],
            [
              "autoplay",
              "",
              "loop",
              "",
              "muted",
              "",
              "playsinline",
              "",
              "width",
              "100%",
              "height",
              "100%",
              "src",
              "webm/webm_phone_1.webm",
              1,
              "h-full",
              "max-h-[480px]",
              "w-full",
            ],
            [
              1,
              "text-secondary-blue",
              "text-base",
              "font-bold",
              "tracking-normal",
              "sm:text-lg",
            ],
            [1, "text-2xl", "font-bold", "lg:text-5xl", 3, "innerHTML"],
            [1, "text-muted", "text-lg"],
          ]
        );
      },
      template: function (a, r) {
        (a & 1 &&
          (l(0, "div", 5, 0)(2, "div", 6)(3, "span", 7),
          Y(4, 3),
          p(),
          l(5, "h4", 8),
          V(6, 4),
          f(7, "br"),
          X(),
          p()(),
          l(8, "div", 9)(9, "div", 10, 1),
          vt(11, xl, 7, 4, "div", 11, yn),
          p(),
          l(13, "div", 12)(14, "video", 13, 2),
          y(16, " Unable to play the video "),
          p()()()()),
          a & 2 && (F(11), yt(r.componentProps)));
      },
      dependencies: [ie, ho, We],
      styles: [
        ".scroll-container__item[_ngcontent-%COMP%]{display:flex;flex-direction:column;gap:1rem;padding:1.5rem;letter-spacing:-.05em}@media (min-width: 1024px){.scroll-container__item[_ngcontent-%COMP%]{gap:1.5rem;padding:3rem 4rem}}@media (min-width: 1280px){.scroll-container__item[_ngcontent-%COMP%]{padding:3rem 7rem}}",
      ],
      changeDetection: 0,
    });
  }
  return c;
})();
var Cl = (c) => ({ "rotate-180": c }),
  Al = (c, E) => ({ "grid-rows-[1fr]": c, "grid-rows-[0fr]": E });
function Tl(c, E) {
  if (c & 1) {
    let i = Ht();
    (l(0, "div", 14)(1, "div", 15),
      pe("click", function () {
        let r = se(i).$implicit;
        return ce((r.expanded = !r.expanded));
      })("mouseenter", function () {
        let r = se(i).$implicit,
          S = _e();
        return ce(!S.isMobile && (r.expanded = !0));
      })("mouseleave", function () {
        let r = se(i).$implicit,
          S = _e();
        return ce(!S.isMobile && (r.expanded = !1));
      }),
      l(2, "div", 16),
      f(3, "img", 17),
      l(4, "h3", 18),
      y(5),
      p()(),
      l(6, "div", 19)(7, "div", 20),
      Zr(),
      l(8, "svg", 21),
      f(9, "path", 22),
      p()()()(),
      ea(),
      l(10, "div", 23)(11, "div", 24),
      f(12, "p", 25),
      p()()());
  }
  if (c & 2) {
    let i = E.$implicit;
    (F(3),
      M("src", i.icon, Ot),
      F(2),
      Rt(i.name),
      F(2),
      M("ngClass", ri(5, Cl, i.expanded)),
      F(3),
      M("ngClass", En(7, Al, i.expanded, !i.expanded)),
      F(2),
      M("innerHTML", i.desc, vn));
  }
}
var Ta = (() => {
  class c {
    cdr = De(bn);
    expanded = !1;
    get isMobile() {
      return window.innerWidth < 1024;
    }
    platformProperties = [
      {
        icon: "svg/join-community/building.svg",
        name: "Properti premium",
        desc: "Akses jenis real estate yang biasanya diperuntukkan bagi orang kaya \u2014 vila mewah, apartemen tepi pantai, dan properti dengan ROI tinggi di Bali dan Indonesia. Dikurasi oleh para ahli, setiap kesepakatan dimulai dari hanya 150.000 Rp. Ini adalah kesempatan Anda untuk memiliki apa yang dulunya tidak terjangkau - tanpa perlu membeli seluruh properti.",
      },
      {
        icon: "svg/join-community/bank-note.svg",
        name: "Penghasilan pasif",
        desc: "Uang Anda mulai bekerja sejak hari pertama. Dapatkan hasil sewa bulanan dan pertumbuhan modal dari penjualan kembali - sepenuhnya otomatis dan didistribusikan langsung ke dompet Anda. Tidak ada agen. Tidak ada penundaan. Hanya penghasilan yang konsisten saat Anda tidur.",
      },
      {
        icon: "svg/join-community/file-shield.svg",
        name: "Investasi yang diaktakan",
        desc: "Setiap saham yang Anda pegang dijamin oleh kontrak legal dan disahkan oleh notaris di yurisdiksi Indonesia. Ini bukan hanya sebuah kontrak - ini adalah kepemilikan saham dengan perlindungan hukum dan transparansi blockchain. Kami telah menghilangkan ketidakpastian, memberikan Anda ketenangan dalam setiap transaksi.",
      },
      {
        icon: "svg/join-community/coins-swap.svg",
        name: "Jalan keluar yang mudah",
        desc: "Anda memegang kendali penuh. Ingin menjual bagian Anda di properti? Daftarkan kapan saja di pasar jual beli P2P bawaan kami. Tanpa dokumen, tanpa perantara, tanpa penjaga gerbang - cukup klik, konfirmasi, dan keluar saat Anda memutuskan. Ini adalah investasi Anda. Anda yang menentukan berapa lama Anda akan menahannnya - atau kapan Anda akan mencairkannya.",
      },
      {
        icon: "svg/join-community/presentation-chart.svg",
        name: "Dasbor investor",
        desc: "Lacak semuanya di satu tempat. Lihat investasi Anda berkembang, pantau dividen, lihat pembayaran yang akan datang, dan buat alokasi baru - semuanya dalam dasbor yang bersih dan intuitif. Ini adalah pusat komando pribadi Anda untuk membangun kekayaan jangka panjang, tanpa spreadsheet atau stres.",
      },
      {
        icon: "svg/join-community/users-check.svg",
        name: "Akses masyarakat",
        desc: "Bergabunglah dengan jaringan global investor yang berpikiran sama - mulai dari pemula hingga profesional berpengalaman. Berpartisipasilah dalam sesi live, berbagi strategi, mengajukan pertanyaan, dan mengakses penawaran awal sebelum saham tersebut go public. Anda tidak hanya berinvestasi. Anda menjadi bagian dari sebuah gerakan, bagian dari jaringan yang kaya.",
      },
      {
        icon: "svg/join-community/coins-stacked.svg",
        name: "Pertumbuhan kekayaan",
        desc: "Investasi pecahan memungkinkan Anda membangun portofolio yang kuat dan terdiversifikasi tanpa mengunci semua modal Anda dalam satu transaksi. Seiring waktu, Anda meningkatkan kepemilikan Anda di berbagai aset premium - meningkatkan kekayaan Anda lebih cepat daripada tabungan tradisional atau permainan berisiko. Inilah investasi masa depan - lebih cerdas, lebih aman, dan bersama-sama.",
      },
      {
        icon: "svg/join-community/globe.svg",
        name: "Kebebasan",
        desc: "Lupakan mengelola penyewa, membayar pengacara, atau stres karena dokumen. POOOL menangani pekerjaan berat sehingga Anda tidak perlu melakukannya - mulai dari kurasi kesepakatan hingga pembayaran. Anda berinvestasi. Anda yang mengontrol. Kami melakukan sisanya. Kepemilikan total, tanpa kerumitan.",
      },
    ];
    ngAfterViewInit() {
      this.isMobile &&
        ((this.platformProperties[0].expanded = !0), this.cdr.detectChanges());
    }
    static ɵfac = function (a) {
      return new (a || c)();
    };
    static ɵcmp = J({
      type: c,
      selectors: [["pool-land-join-community"]],
      decls: 20,
      vars: 0,
      consts: () => {
        let i;
        i = "Bergabung";
        let a;
        return (
          (a = "komunitas investor"),
          [
            i,
            a,
            [
              1,
              "flex",
              "items-center",
              "justify-center",
              "py-[90px]",
              "lg:py-[120px]",
            ],
            [
              1,
              "flex",
              "flex-col",
              "items-center",
              "gap-10",
              "px-4",
              "lg:gap-14",
              "lg:px-32",
              "xl:px-52",
            ],
            [
              1,
              "flex",
              "flex-col",
              "items-center",
              "justify-center",
              "gap-4",
              "lg:gap-14",
            ],
            [
              "bubbleAnimation",
              "",
              1,
              "text-center",
              "text-4xl",
              "font-extrabold",
              "uppercase",
              "tracking-tighter",
              "lg:text-7xl",
            ],
            [
              1,
              "text-primary-blue",
              "mx-4",
              "inline-flex",
              "w-min",
              "items-center",
              "justify-start",
              "gap-2",
            ],
            [
              1,
              "-ml-1",
              "inline-flex",
              "h-[28px]",
              "min-w-[60px]",
              "lg:-ml-2",
              "lg:-mr-2",
              "lg:h-[56px]",
              "lg:min-w-[144px]",
              "xl:-mr-3",
            ],
            [
              "src",
              "/svg/world-map/usa.svg",
              "alt",
              "USA flag",
              1,
              "-mr-[8px]",
              "lg:-mr-3",
            ],
            [
              "src",
              "/png/flag-estonia-circle.webp",
              "alt",
              "Estonia flag",
              1,
              "-mr-[8px]",
              "lg:-mr-3",
            ],
            ["src", "/svg/world-map/japan.svg", "alt", "Japan flag"],
            [1, "block", "sm:hidden"],
            ["src", "/webp/poool-community.webp", "alt", "POOOL community"],
            [
              1,
              "flex",
              "flex-col",
              "items-start",
              "justify-center",
              "gap-4",
              "lg:grid",
              "lg:grid-cols-4",
            ],
            [
              1,
              "hover:bg-secondary-green",
              "group",
              "flex",
              "w-full",
              "flex-col",
              "justify-center",
              "rounded-xl",
              "border",
              "border-[#98FB96]",
              "bg-[#F3F8FB]",
              "p-2",
              "lg:max-w-[220px]",
            ],
            [
              "role",
              "button",
              1,
              "flex",
              "cursor-pointer",
              "items-center",
              "justify-center",
              "gap-1",
              "lg:justify-between",
              3,
              "click",
              "mouseenter",
              "mouseleave",
            ],
            [1, "flex", "items-center", "gap-2"],
            ["alt", "Icon", 1, "h-6", "w-6", 3, "src"],
            [1, "text-sm", "font-medium"],
            [
              1,
              "relative",
              "flex",
              "h-6",
              "w-6",
              "flex-shrink-0",
              "items-center",
              "justify-center",
              "rounded-full",
            ],
            [
              1,
              "transition-transform",
              "duration-300",
              "ease-out",
              3,
              "ngClass",
            ],
            [
              "xmlns",
              "http://www.w3.org/2000/svg",
              "fill",
              "none",
              "viewBox",
              "0 0 24 24",
              "stroke",
              "currentColor",
              "stroke-width",
              "2",
              1,
              "h-5",
              "w-5",
              "text-[#98FB96]",
              "group-hover:text-black",
            ],
            [
              "stroke-linecap",
              "round",
              "stroke-linejoin",
              "round",
              "d",
              "M19 9l-7 7-7-7",
            ],
            [
              1,
              "grid",
              "transition-[grid-template-rows]",
              "duration-300",
              3,
              "ngClass",
            ],
            [1, "overflow-hidden"],
            [
              1,
              "whitespace-pre-line",
              "px-2",
              "py-2",
              "text-[12px]",
              "leading-none",
              3,
              "innerHTML",
            ],
          ]
        );
      },
      template: function (a, r) {
        (a & 1 &&
          (l(0, "section", 2)(1, "div", 3)(2, "div", 4)(3, "div", 5),
          ni(4),
          Y(5, 0),
          ii(),
          l(6, "div", 6),
          y(7, " w "),
          l(8, "div", 7),
          f(9, "img", 8)(10, "img", 9)(11, "img", 10),
          p(),
          y(12, " rldwide "),
          p(),
          f(13, "br", 11),
          ni(14),
          Y(15, 1),
          ii(),
          p(),
          f(16, "img", 12),
          p(),
          l(17, "div", 13),
          vt(18, Tl, 13, 10, "div", 14, yn),
          p()()()),
          a & 2 && (F(18), yt(r.platformProperties)));
      },
      dependencies: [ie, li, We],
      encapsulation: 2,
      changeDetection: 0,
    });
  }
  return c;
})();
var wa = [
  {
    name: "Jonas Freiwald",
    title: "Co-Founder and Co-CEO",
    bio: "Jonas Freiwald adalah Co-Founder dan Co-CEO POOOL, serta Co-Founder Bali Invest - sebuah perusahaan pengembangan real estate butik di Indonesia yang memainkan peran penting dalam menguji coba model investasi POOOL. Berasal dari Jerman, Jonas memiliki latar belakang yang kaya dan lintas disiplin ilmu yang mencakup perhotelan, kepemimpinan tim internasional, dan pengembangan usaha.<br /><br />Perjalanan profesionalnya dimulai di industri kuliner dan hotel, di mana ia mengasah disiplin operasional, pengalaman pelanggan, dan komunikasi dalam lingkungan berkinerja tinggi. Bertransisi ke dunia investasi dan pembangunan bisnis, Jonas membenamkan diri dalam perdagangan, keuangan, dan real estat - mengembangkan pemahaman yang mendalam tentang dinamika modal dan eksekusi proyek di pasar negara berkembang.<br /><br />Sebagai Co-Founder Bali Invest, Jonas telah memimpin akuisisi, pengembangan, dan pemasaran beberapa proyek vila di seluruh Bali - membangun platform tepercaya bagi investor internasional. Dalam Bali Invest inilah konsep POOOL pertama kali diuji: menokenkan akses ke real estat premium dan memvalidasi permintaan untuk kepemilikan fraksional. Tulang punggung operasional ini memberi POOOL lingkungan yang hidup untuk menyempurnakan proposisi nilainya, struktur hukum, dan perjalanan investor - menjembatani teori dengan bukti.<br /><br />Di POOOL, Jonas sekarang berfokus pada eksekusi operasional, arsitektur penjualan, dan pengembangan jaringan investor. Pengalaman gandanya sebagai pengembang dan investor memungkinkannya untuk melihat kedua sisi pasar - membuatnya berperan penting dalam membangun sistem, hubungan, dan kredibilitas yang diperlukan untuk meningkatkan skala platform di seluruh Asia Tenggara dan sekitarnya.",
  },
  {
    name: "Nikita Kokhanevych",
    title: "CTO, Blockchain",
    bio: "Nikita Kokhanevych dan Mykyta Hlukhuvskyi menjabat sebagai Co-CTO POOOL, yang memimpin arsitektur, keamanan, dan evolusi teknologi platform. Dengan lebih dari 7 tahun pengalaman dalam pengembangan Web2 dan Web3, mereka membawa kombinasi langka antara keahlian blockchain yang mendalam dan pemikiran produk SaaS yang diskalakan - penting untuk membangun infrastruktur investasi generasi berikutnya dari POOOL.<br /><br />Sebelum bergabung dengan POOOL, Nikita dan Mykyta telah memberikan solusi pengembangan khusus di puluhan proyek kripto dengan beban tinggi, pasar online, bot perdagangan, dan platform terintegrasi AI. Kekuatan utama mereka terletak pada menjembatani logika bisnis tradisional dengan teknologi terdesentralisasi - membuat sistem tokenisasi yang lantas, pasar NFT, dan dasbor keuangan yang kuat dan berpusat pada pengguna.<br /><br />Di POOOL, mereka mengawasi tim produk khusus yang terdiri dari insinyur backend Web3, pengembang frontend UI/UX, dan pemimpin operasi R&D senior. Fokus mereka: untuk memastikan bahwa kepemilikan token, logika kontrak pintar, dan aliran investor bekerja bersama sebagai satu ekosistem yang terpadu dan aman. Baik mengintegrasikan aset real estat atau memungkinkan kesepakatan usaha fraksional, Nikita dan Mykyta memastikan POOOL berjalan sebagai mesin investasi berkinerja tinggi - dengan keandalan, skala, dan teknologi yang tahan terhadap masa depan sebagai intinya.",
  },
  {
    name: "Sean Reno",
    title: "Head of Indonesian Partnerships",
    bio: "Sean Reno adalah Kepala Kemitraan Indonesia di POOOL dan pemangku kepentingan utama, memegang 3% saham ekuitas di perusahaan bersama rekan bisnisnya Patrick Werner melalui perusahaan TEKNIKA PropTech Hub. Dengan akar yang kuat di bidang PropTech dan inovasi real estate Indonesia, Sean memainkan peran penting dalam menjembatani hubungan institusional, dialog dengan pemerintah, dan pengembangan infrastruktur digital untuk ekspansi POOOL di seluruh Asia Tenggara.<br /><br />Sebelum bergabung dengan POOOL, Sean membangun reputasi sebagai suara nasional dalam transformasi properti digital, menjabat sebagai Wakil Sekretaris Jenderal di Dewan Perwakilan Real Estat Indonesia (DPP REI) dan Presiden Direktur di TEKNIKA PropTech Hub. Pengaruhnya meluas hingga ke panggung real estat global melalui kepemimpinannya di FIABCI\u2014Federasi Real Estat Internasional\u2014di mana ia telah memegang berbagai peran.<br /><br />Di POOOL, Sean memanfaatkan perpaduan unik antara pengaruh sektor swasta, publik, dan global sektor untuk membangun aliansi strategis jangka panjang yang membuka modal, legitimasi, dan inovasi.",
  },
  {
    name: "Monique Howeth",
    title: "Venture investor",
    bio: "Monique Howeth adalah investor ventura dan penasehat strategis di POOOL, di mana ia memegang 5% saham ekuitas dan memainkan peran kunci dalam membentuk strategi modal perusahaan, penyelarasan peraturan, dan peningkatan skala internasional. Dengan lebih dari 25 tahun pengalaman di industri real estat AS dan latar belakang yang kuat dalam bisnis yang didukung oleh ventura, Monique membawa keahlian operasional yang tak tertandingi dan disiplin investor ke ruang rapat POOOL.<br /><br />Karirnya mencakup pengembangan real estat perumahan dan komersial, pertumbuhan waralaba, dan pengelolaan dana. Sebagai mantan eksekutif di Crunch Fitness, ia memainkan peran penting dalam menskalakan salah satu merek kebugaran paling terkenal di Amerika, mengawasi ekspansi multi-unit dan operasi waralaba. Pengalaman kepemimpinan langsung ini, dikombinasikan dengan keahlian modal ventura, memberinya lensa ganda yang langka: pengetahuan operasional berbasis aset yang mendalam dan pandangan strategis tentang pasar modal.<br /><br />Di POOOL, Monique bertindak sebagai investor dan penasehat senior\u2014memandu masuknya platform ke pasar-pasar yang belum berkembang seperti Indonesia, memastikan kesesuaian dengan peraturan, dan menyelaraskan model bisnis untuk adopsi institusional jangka panjang. Keterlibatannya menggarisbawahi keyakinan dalam tesis POOOL: bahwa real estat fraksional yang didukung blockchain adalah masa depan investasi properti global.",
  },
  {
    name: "Dmitry Sikorski",
    title: "Co-Founder and Co-CEO",
    bio: "<strong>Ukrainian entrepreneur, visionary & strategist. A founder who turns chaos into structure, ideas into capital, and trust into unstoppable teams.</strong><br /><br /><strong>NOT JUST THE CREATOR OF POOOL \u2014 THE ARCHITECT OF EVERYTHING THAT MADE IT POSSIBLE</strong><br /><br />Today, Dmitry Sikorski is the Co-Founder and Co-CEO of <strong>POOOL</strong> \u2014 Indonesia's first investment platform for tokenized real estate, startups, and commodities. But behind this technological and strategic success is a decade-long journey of risk, betrayal, reinvention, and unshakable belief.<br /><br />Dmitry didn't just come up with a business model \u2014 <strong>he built an entire ecosystem around it: the team, the product, the regulatory roadmap, the investor trust, the government relationships, and the billion-dollar vision.</strong><br /><br />Dmitry \u2014 and the team he personally brought together \u2014 are the reason why key players aligned around POOOL. From Indonesia's top financial regulator OJK, the Ministry of Housing, and the national stock exchange IDX, to corporations like Pertamina, global associations like FIABCI, leading real estate developers, and international investors. <strong>People didn't just follow the project \u2014 they followed the person willing to put everything on the line to make it real.</strong><br /><br /><strong>2015: First Startup. First Success. First Collapse.</strong><br /><br />Dmitry launched his first startup in <strong>2015</strong> \u2014 a tech-platform for e-commerce called <strong>SMR (Social Media Recruiter)</strong>. It combined lead generation tools for main social media, a drag-and-drop landing page builder, and an educational module for entrepreneurs.<br /><br />The project quickly gained momentum \u2014 reaching the <strong>Top 100 Forbes Russia</strong> <strong>(Young Billionaire School)</strong>, selling <strong>over 10,000 subscriptions</strong>, generating <strong>$150,000 in revenue</strong>, and raising <strong>$100,000 in a pre-seed round</strong> within just six months.<br /><br />But at the peak, his co-founder betrayed him \u2014 stealing access to the platform, locking out the team, rerouting payments, and crashing the company.<br /><br /><strong>It could've been the end. But for Dmitry, it was the ignition point.</strong><br /><br /><strong>2016\u20132022: Agency Years. Banda. Strategic Depth.</strong><br /><br />Instead of quitting, Dmitry gathered a small team and launched his own marketing agency \u2014 scaling campaigns across fintech, e-commerce, agriculture, and large-scale events. In addition to his work in tech and marketing, Dmitry successfully launched several e-commerce businesses using the dropshipping model on Shopify. These ventures gave him hands-on experience in global logistics, performance marketing, and customer psychology \u2014 further sharpening his skills in building lean, scalable systems that convert attention into revenue.<br /><br />In <strong>2021</strong>, he became a strategist at <strong>Banda Agency</strong> \u2014 <strong>ranked the #1 creative agency in the world by EFFIE Global (2020)</strong>. There, he mastered the art of turning brands into market leaders \u2014 blending bold creativity with precise business logic.<br /><br />He didn't just help businesses grow \u2014 he helped them dominate.<br /><br /><strong>2022\u20132024: Bali. Real Estate. System Thinking.</strong><br /><br />Upon relocating to Indonesia, Dmitry began immersing himself in the Balinese real estate market \u2014 analyzing ROI, developer structures, legal limitations for foreigners, and high-return opportunities. He started investing personally and built deep relationships with local industry leaders.<br /><br />This period sparked the idea of POOOL \u2014 <strong>a platform that gives global investors access to premium, income-generating assets in Indonesia from as little as $1</strong>.<br /><br />Where others saw complexity, he saw infrastructure.<br />Where others saw regulation, he saw opportunity.<br />Where others hesitated, <strong>he built.</strong><br /><br /><strong>2024\u20132025: Launching POOOL \u2014 the Convergence of Everything That Came Before</strong><br /><br />POOOL is not just a company. It's <strong>the culmination of Dmitry's entire journey</strong>.<br /><br />\u2022 He structured the legal framework (PT PMA + SPV) and leads regulatory approval through OJK's sandbox.<br />\u2022 He defined the product architecture \u2014 from fractional investing to future digital banking rails.<br />\u2022 He assembled a world-class team \u2014 from European strategists to Indonesian compliance experts and PropTech veterans.<br />\u2022 He secured meetings with ministries, struck partnerships with developers, and onboarded early investors.<br /><br />Today, Dmitry doesn't just lead POOOL. <strong>He is building the foundation of a new investment economy in Southeast Asia.</strong><br /><br /><strong>Mission: Turn tokenization into Southeast Asia's capital engine</strong><br /><br />For Dmitry, POOOL isn't \"just another blockchain startup.\" It's a <strong>technology of access</strong>. A capital formation machine. A way to open previously gated investment opportunities \u2014 from real estate to private equity \u2014 to the global middle class.",
  },
  {
    name: "Mykyta Hlukhuvskyi",
    title: "CTO, Back-end",
    bio: "Nikita Kokhanevych dan Mykyta Hlukhuvskyi menjabat sebagai Co-CTO POOOL, yang memimpin arsitektur, keamanan, dan evolusi teknologi platform. Dengan lebih dari 7 tahun pengalaman dalam pengembangan Web2 dan Web3, mereka membawa kombinasi langka antara keahlian blockchain yang mendalam dan pemikiran produk SaaS yang diskalakan - penting untuk membangun infrastruktur investasi generasi berikutnya dari POOOL.<br /><br />Sebelum bergabung dengan POOOL, Nikita dan Mykyta telah memberikan solusi pengembangan khusus di puluhan proyek kripto dengan beban tinggi, pasar online, bot perdagangan, dan platform terintegrasi AI. Kekuatan utama mereka terletak pada menjembatani logika bisnis tradisional dengan teknologi terdesentralisasi - membuat sistem tokenisasi yang lantas, pasar NFT, dan dasbor keuangan yang kuat dan berpusat pada pengguna.<br /><br />Di POOOL, mereka mengawasi tim produk khusus yang terdiri dari insinyur backend Web3, pengembang frontend UI/UX, dan pemimpin operasi R&D senior. Fokus mereka: untuk memastikan bahwa kepemilikan token, logika kontrak pintar, dan aliran investor bekerja bersama sebagai satu ekosistem yang terpadu dan aman. Baik mengintegrasikan aset real estat atau memungkinkan kesepakatan usaha fraksional, Nikita dan Mykyta memastikan POOOL berjalan sebagai mesin investasi berkinerja tinggi - dengan keandalan, skala, dan teknologi yang tahan terhadap masa depan sebagai intinya.",
  },
  {
    name: "Patrick Werner",
    title: "Direktur Lokal",
    bio: "Patrick Werner is the Head of External Affairs at POOOL and a strategic stakeholder through TEKNIKA PropTech Hub, where he co-leads cross-sector initiatives driving digital transformation across Southeast Asia's property and sustainability landscape. With a background in mass communication and deep expertise in stakeholder engagement, Patrick plays a central role in expanding POOOL's institutional footprint within the Indonesian market and beyond.<br /><br />Before joining POOOL, Patrick designed and led programs at the intersection of real estate, agriculture, and community development\u2014building bridges between innovation and inclusion. As Co-Founder and Director of TEKNIKA PropTech Hub, he developed long-term collaborations with government ministries, academic institutions, and private sector leaders\u2014aligning digital infrastructure and proptech innovation with Indonesia's national development priorities.<br /><br />At POOOL, Patrick brings a rare blend of grassroots intuition and institutional diplomacy. He leads the platform's engagement with regulators, real estate associations, and mission-aligned partners\u2014positioning POOOL not just as a technology platform, but as a trusted ecosystem for inclusive, blockchain-enabled property access.<br /><br />Patrick's vision is to democratize real estate investment\u2014making it more transparent, sustainable, and accessible to a wider range of communities through the power of technology and collective ownership.",
  },
  {
    name: "Jack Hurman",
    title: "Head of Global Partnerships",
    bio: "Jack adalah Kepala Kemitraan Global di POOOL, di mana ia memimpin kolaborasi strategis, usaha patungan, dan pengembangan ekosistem. Dengan lebih dari 7 tahun pengalaman dalam industri keuangan dan brokerage, Jack brings deep commercial understanding untuk strategi kemitraan POOOL\u2014menjembatani kesenjangan antara model keuangan tradisional dan ekosistem digital yang berkembang.<br /><br />Sebelum bergabung dengan POOOL, Jack membangun dan menskalakan beberapa tim berkinerja tinggi dalam ruang FX dan brokerage, spesialisasi dalam pembayaran lintas batas, akuisisi klien, dan manajemen hubungan strategis. Track record-nya dalam bernegosiasi kesepakatan bernilai tinggi dan memupuk kepercayaan klien jangka panjang telah membentuk filosofi kemitraannya: penyelarasan, transparansi, dan nilai bersama.<br /><br />Di POOOL, Jack menggabungkan keahlian keuangannya dengan pendekatan berpikiran maju terhadap kolaborasi\u2014memupuk hubungan yang mempercepat pertumbuhan, mendukung inovasi, dan memperluas kehadiran global platform. Passionate tentang membangun struktur win-win, Jack memainkan peran penting dalam memastikan kemitraan POOOL tidak hanya strategis, tetapi juga berkelanjutan dan berdampak.",
  },
  {
    name: "Daniel Todorov",
    title: "Head of Design",
    bio: "Daniel adalah Kepala Desain di POOOL, di mana ia memimpin fungsi desain dan mengawasi komunikasi visual, desain produk, dan kolaborasi lintas fungsi dengan pengembang frontend. Mengelola tim yang terdiri dari desainer produk, desainer grafis, dan insinyur frontend, ia memastikan eksekusi pengalaman yang berpusat pada pengguna dan identitas visual yang konsisten di seluruh platform.<br/><br/>Dengan lebih dari 8 tahun pengalaman di bidang UX/UI dan desain produk, Daniel membangun keahliannya di agensi desain terbaik di seluruh wilayah CIS, memberikan antarmuka digital yang intuitif dan berdampak tinggi. Kekuatannya terletak pada mengubah kerumitan menjadi kejelasan \u2014 merancang sistem yang elegan dan efektif.<br/><br/>Daniel melakukan pendekatan desain dengan pola pikir yang strategis dan sistematis, menggabungkan riset pengguna yang mendalam dan wawasan berbasis data dengan pemecahan masalah yang kreatif. Filosofinya adalah bahwa desain yang bagus menjembatani kebutuhan pengguna dan tujuan bisnis, mendorong pertumbuhan dan kepuasan pelanggan.<br/><br/>Di POOOL, Daniel memainkan peran penting dalam membentuk bahasa desain platform, mengelola alur kerja pengembangan-desain, dan membangun budaya tim kolaboratif. Kepemimpinannya menjembatani kreativitas dan teknologi \u2014 memungkinkan POOOL untuk berkembang dengan konsistensi, dampak, dan inovasi",
  },
  {
    name: "Victoria Andreieva",
    title: "Investors Relations",
    bio: "Victoria Anreieva is leading Investor Relations at POOOL, the pioneering platform for fractional real estate and venture investment in Southeast Asia. With a proven track record of raising over $20 million across leading crypto and venture funds, Victoria brings elite capital experience and a sharp understanding of what drives large investors. <br /><br />Over the past three years, she worked directly with Ryan Fang, co-founder of ANKR Network and co-founder of Symbolic Capital (Liquid Venture Fund), created in partnership with Sandeep Nailwal, co-founder of Polygon. As part of their core fundraising team, Victoria helped structure deals, navigate institutional relationships, and secure capital from top-tier players in the Web3 space. <br /><br />Her close business-relations with Ryan and his network opened doors to the world of billion-dollar venture funds and taught her the mechanics of capital at scale. Today, while continuing her work with Symbolic Capital, Victoria has joined POOOL out of conviction: in the team, in the product, and in the massive potential of tokenized real estate in Indonesia. <br /><br />At POOOL, Victoria leads investor relations \u2014 bridging global capital with high-growth assets in Asia. She is also an active investor in real estate herself, with a focus on Bali, giving her a unique dual perspective: both as a professional capital raiser and as a buyer of the very assets POOOL is democratizing. Her mission is to open this opportunity to thousands of investors around the world \u2014 starting now, at the pre-seed stage.",
  },
  {
    name: "Nataly Vovque",
    title: "Branding",
    bio: "Nataly is the Head of Branding at POOOL, where she leads the company's visual identity, brand strategy, and global marketing efforts. With over 10 years of experience in branding, marketing, and design, Nataly brings a rare combination of strategic depth and creative firepower to POOOL's growth engine.<br/><br/>Before joining POOOL, Nataly served as a Senior Brand Strategist at Banda Agency \u2014 the most awarded creative agency in Ukraine and one of the most respected branding firms in Eastern Europe. Banda is known for crafting bold national campaigns, building unicorn brand identities, and consistently ranking among the world's most effective agencies (including EFFIE #1). During her time there, Nataly helped orchestrate large-scale brand launches and award-winning strategic platforms, honing the ability to turn abstract ideas into movements that convert.<br/><br/>Nataly's career spans residential and commercial real estate development, franchise growth, and fund management. As a former executive at Crunch Fitness, she played a pivotal role in scaling one of America's most recognizable fitness brands, overseeing multi-unit expansion and franchise operations. This hands-on leadership experience, combined with her branding expertise, gives her a rare dual-lens: deep asset-based operational know-how and a strategic view of capital markets.<br/><br/>At POOOL, Nataly acts as both investor and senior advisor\u2014guiding the platform's entry into frontier markets like Indonesia, ensuring regulatory fit, and aligning the business model for long-term institutional adoption. Her involvement underscores confidence in POOOL's thesis: that fractional, blockchain-backed real estate is the future of global property investment.",
  },
  {
    name: "Jonathan Rizky",
    title: "Kepala Kepatuhan",
    bio: "Jonathan Rizky \u2014 Kepala Kepatuhan, POOOL<br/><br/>Jonathan Rizky adalah profesional berpengalaman dengan lebih dari 10 tahun pengalaman di industri perbankan, fintech, dan cryptocurrency. Keahliannya mencakup strategi bisnis, manajemen risiko, dan kepatuhan regulasi, dengan rekam jejak yang terbukti dalam membimbing perusahaan melalui proses perizinan yang kompleks dengan regulator keuangan.<br/><br/>Sepanjang kariernya, Jonathan telah memegang peran kepatuhan senior di lembaga-lembaga terkemuka termasuk CIMB Bank, Digiasia Bios (FaaS), dan CFX, di mana ia memainkan peran kunci dalam mendapatkan persetujuan regulasi dan memastikan kepatuhan terhadap standar kepatuhan internasional.<br/><br/>Di POOOL, Jonathan memimpin strategi kepatuhan dan regulasi, memastikan platform beroperasi dalam standar transparansi tertinggi, perlindungan investor, dan keselarasan dengan otoritas keuangan Indonesia seperti OJK dan Kominfo. Pengalamannya dalam perizinan bursa kripto dan operator fintech menempatkan POOOL di garis depan pasar RWA (Aset Dunia Nyata yang Ditokenisasi) yang teregulasi di Indonesia.",
  },
  {
    name: "Khairatul Raudah",
    title: "Asisten Perusahaan",
    bio: 'Khairatul Raudah \u2014 Asisten Perusahaan, POOOL<br/><br/>Di POOOL, Khairatul memainkan peran penting dalam mendukung proses operasional dan strategis di seluruh perusahaan. Dengan keterampilan organisasi dan komunikasi yang kuat, ia memastikan alur kerja antar departemen berjalan lancar dan tim kepemimpinan dapat fokus pada pengembangan platform.<br/><br/>Filosofinya sederhana namun kuat:<br/><br/>"Di POOOL, saya menemukan lebih dari sekadar peran \u2014 saya menemukan tempat di mana kolaborasi dan kepemilikan benar-benar penting. Setiap proyek adalah kesempatan untuk mendukung visi kami dalam membangun komunitas yang kuat dan berkelanjutan."<br/><br/>Pekerjaan Khairatul mewujudkan semangat keandalan dan dedikasi, membantu menyelaraskan operasi sehari-hari dengan misi POOOL yang lebih luas: membuka akses ke aset premium dan menciptakan nilai jangka panjang bagi investor dan komunitas.',
  },
];
var wl = ["scrollingWrapper"],
  Dl = ["scrollingContent"],
  Ml = () => ({}),
  Nl = (c, E) => E.image;
function Rl(c, E) {
  if (c & 1) {
    let i = Ht();
    (l(0, "div", 13),
      pe("click", function () {
        let r = se(i).$index,
          S = _e();
        return ce(S.clickCard(r));
      })("keydown.enter", function () {
        se(i);
        let r = _e();
        return ce(r.showCompanyInfoDialog.set(!1));
      })("keydown.space", function () {
        se(i);
        let r = _e();
        return ce(r.showCompanyInfoDialog.set(!1));
      }),
      l(1, "div", 14),
      f(2, "img", 15),
      l(3, "div", 16)(4, "p", 17)(5, "span", 18)(6, "span", 19),
      y(7, "\u275D"),
      p(),
      y(8),
      p()()()(),
      l(9, "div", 20)(10, "h2", 21)(11, "span"),
      y(12),
      bt(13, "uppercase"),
      p(),
      l(14, "span"),
      y(15),
      bt(16, "uppercase"),
      p()(),
      l(17, "span", 22),
      y(18),
      p()()());
  }
  if (c & 2) {
    let i = E.$implicit;
    (F(2),
      M("src", i.image, Ot)("alt", i.name)(
        "ngStyle",
        i.imageStyle || oi(11, Ml),
      ),
      F(6),
      ne(" ", i.quote, " "),
      F(4),
      Rt(zn(13, 7, i.name)),
      F(3),
      Rt(zn(16, 9, i.surname)),
      F(3),
      ne(" ", i.position, " "));
  }
}
function Ll(c, E) {
  if (c & 1) {
    let i = Ht();
    (l(0, "div", 23),
      pe("click", function () {
        se(i);
        let r = _e();
        return ce(r.showCompanyInfoDialog.set(!1));
      })("keydown.enter", function () {
        se(i);
        let r = _e();
        return ce(r.showCompanyInfoDialog.set(!1));
      }),
      l(1, "div", 24),
      pe("click", function (r) {
        return (se(i), ce(r.stopPropagation()));
      })("keydown.enter", function (r) {
        return (se(i), ce(r.stopPropagation()));
      }),
      l(2, "div", 25),
      f(3, "img", 26),
      l(4, "div", 27)(5, "h2", 28),
      y(6),
      p(),
      l(7, "h3", 29),
      y(8),
      l(9, "span", 30),
      y(10, "POOOL"),
      p()()(),
      f(11, "p", 31)(12, "div", 32),
      p(),
      l(13, "button", 33),
      pe("click", function () {
        se(i);
        let r = _e();
        return ce(r.showCompanyInfoDialog.set(!1));
      })("keydown.enter", function () {
        se(i);
        let r = _e();
        return ce(r.showCompanyInfoDialog.set(!1));
      }),
      l(14, "span", 34),
      y(15, "\xD7"),
      p()()()());
  }
  if (c & 2) {
    let i = _e();
    (F(6),
      ne(" ", i.getClickedCardReview().name, " "),
      F(2),
      ne(" ", i.getClickedCardReview().title, " at "),
      F(3),
      M("innerHTML", i.getClickedCardReview().bio, vn));
  }
}
var Da = (() => {
  class c {
    cdr = De(bn);
    destroyRef = De(ta);
    wrapperRef;
    contentRef;
    showCompanyInfoDialog = Hn(!1);
    currentIdx = Hn(0);
    scrollPosition = Hn(0);
    isDragging = !1;
    hasDraggedDistance = !1;
    startX = 0;
    scrollLeft = 0;
    dragVelocity = 0;
    lastDragX = 0;
    dragAnimationId = null;
    members = [
      {
        name: "Jonas",
        surname: "Freiwald",
        image: "/webp/team/Jonas Freiwald.webp",
        quote:
          "Seluruh Pengalaman Saya di Dunia Properti Mengarah ke Momen Ini - Meluncurkan POOOL.",
        position: "Co-CEO, Founder",
      },
      {
        name: "Dmitry",
        surname: "Sikorski",
        image: "/webp/team/Dmitry Sikorski.webp",
        quote:
          "Saya Tahu Rasanya Membangun dari Nol. POOOL Memberikan Kesempatan Itu untuk Siapa Saja",
        position: "Co-CEO, Founder",
      },
      {
        name: "Sean",
        surname: "Reno",
        image: "/webp/team/Sean Reno.webp",
        quote:
          "Banyak platform menjanjikan kepercayaan \u2014 tapi POOOL benar-benar didukung oleh hukum",
        position: "Government Relations",
      },
      {
        name: "Patrick",
        surname: "Werner",
        image: "/webp/team/Patrick Werner.webp",
        quote:
          "POOOL adalah alat yang bisa membantu siapa pun di seluruh dunia untuk berinvestasi",
        position: "Direktur Lokal",
      },
      {
        name: "Daniel",
        surname: "Todorov",
        image: "/webp/team/Daniel Todorov.webp",
        quote:
          "Semua orang menjual mimpi. Kami membangun POOOL untuk memfraksionalkan yang nyata",
        position: "Head of Design",
      },
      {
        name: "Nikita",
        surname: "Kokhanevych",
        image: "/webp/team/Nikita Kokhanevych.webp",
        quote:
          "Dulu, investasi hanya untuk kalangan kaya. POOOL membongkar aturannya untuk semua orang",
        position: "CTO, Blockchain",
      },
      {
        name: "Mykyta",
        surname: "Hlukhuvskyi",
        image: "/webp/team/Mykyta Hlukhuvskyi.webp",
        quote:
          "Berinvestasi properti sekarang lebih mudah daripada memesan taksi. Itulah POOOL",
        position: "CTO, Back-end",
      },
      {
        name: "Nataly ",
        surname: "Vovque",
        image: "/webp/team/Nataly Vovque.webp",
        quote:
          "Uang lama menguasai gedung-gedung. POOOL membuat uang baru memiliki masa depan.",
        position: "Branding",
      },
      {
        name: "Jonathan",
        surname: "Rizky",
        image: "/png/Jonathan Rizky.webp",
        quote:
          "Bagi saya, POOOL adalah tempat di mana kepatuhan bertemu dengan inovasi \u2014 tempat di mana kepercayaan dibangun, dan regulasi menjadi fondasi untuk pertumbuhan.",
        position: "Compliance Officer",
      },
      {
        name: "Khairatul",
        surname: "Raudah",
        image: "/png/Khai.webp",
        quote:
          "Bagi saya, POOOL adalah tempat di mana ide berubah menjadi tindakan, dan kerja sama tim menciptakan perubahan nyata.",
        position: "Asisten Perusahaan",
        imageStyle: { transform: "scale(1.2) translateY(-10px)" },
      },
    ];
    ngAfterViewInit() {
      (this.initializeDragToScroll(),
        Kr(20)
          .pipe(
            Qr(this.getAllowScroll()),
            Yr(([i, a]) => a),
            si(this.destroyRef),
          )
          .subscribe(() => {
            let i = this.wrapperRef.nativeElement,
              r =
                this.contentRef.nativeElement.offsetWidth / this.members.length,
              S = Math.floor((i.clientWidth + r - 1) / r),
              x = i.scrollWidth - S * r,
              B = r * S,
              te = !(i.scrollLeft < x);
            (this.syncAndUpdateScrollPosition(i.scrollLeft, te ? B : void 0),
              te &&
                (this.scrollPosition.set(i.scrollLeft - B),
                (this.members = [
                  ...this.members.slice(S),
                  ...this.members.slice(0, S),
                ]),
                this.cdr.detectChanges()));
          }));
    }
    initializeDragToScroll() {
      let i = this.wrapperRef.nativeElement;
      (Yt(i, "mousedown")
        .pipe(si(this.destroyRef))
        .subscribe((a) => {
          a.button === 0 &&
            ((this.isDragging = !0),
            (this.hasDraggedDistance = !1),
            (this.startX = a.pageX - i.offsetLeft),
            (this.scrollLeft = i.scrollLeft),
            (this.lastDragX = a.pageX),
            (this.dragVelocity = 0),
            this.dragAnimationId &&
              (cancelAnimationFrame(this.dragAnimationId),
              (this.dragAnimationId = null)),
            (i.style.cursor = "grabbing"),
            (i.style.userSelect = "none"),
            a.preventDefault());
        }),
        Yt(document, "mousemove")
          .pipe(si(this.destroyRef))
          .subscribe((a) => {
            if (!this.isDragging) return;
            a.preventDefault();
            let S = (a.pageX - i.offsetLeft - this.startX) * 1.5,
              x = this.scrollLeft - S;
            (Math.abs(S) > 5 && (this.hasDraggedDistance = !0),
              (this.dragVelocity = a.pageX - this.lastDragX),
              (this.lastDragX = a.pageX),
              (i.scrollLeft = x),
              this.handleDragLoop(i),
              this.scrollPosition.set(i.scrollLeft));
          }),
        Yt(document, "mouseup")
          .pipe(si(this.destroyRef))
          .subscribe(() => {
            this.isDragging &&
              ((this.isDragging = !1),
              (i.style.cursor = "grab"),
              (i.style.userSelect = ""),
              this.applyMomentum());
          }),
        Yt(i, "mouseleave")
          .pipe(si(this.destroyRef))
          .subscribe(() => {
            this.isDragging &&
              ((this.isDragging = !1),
              (i.style.cursor = "grab"),
              (i.style.userSelect = ""),
              this.applyMomentum());
          }),
        (i.style.cursor = "grab"));
    }
    handleDragLoop(i) {
      let r = this.contentRef.nativeElement.offsetWidth / this.members.length,
        S = Math.floor((i.clientWidth + r - 1) / r),
        x = i.scrollWidth - S * r,
        B = r * S;
      i.scrollLeft >= x
        ? ((i.scrollLeft = i.scrollLeft - B),
          (this.scrollLeft = this.scrollLeft - B),
          (this.members = [
            ...this.members.slice(S),
            ...this.members.slice(0, S),
          ]),
          this.cdr.detectChanges())
        : i.scrollLeft <= 0 &&
          ((i.scrollLeft = i.scrollLeft + B),
          (this.scrollLeft = this.scrollLeft + B),
          (this.members = [
            ...this.members.slice(-S),
            ...this.members.slice(0, -S),
          ]),
          this.cdr.detectChanges());
    }
    applyMomentum() {
      let i = this.wrapperRef.nativeElement,
        a = this.dragVelocity * 2,
        r = () => {
          Math.abs(a) > 0.5
            ? ((i.scrollLeft -= a),
              this.handleDragLoop(i),
              this.scrollPosition.set(i.scrollLeft),
              (a *= 0.95),
              (this.dragAnimationId = requestAnimationFrame(r)))
            : (this.dragAnimationId = null);
        };
      Math.abs(a) > 1 && r();
    }
    syncAndUpdateScrollPosition(i, a) {
      let r = i > 0 && Math.abs(this.scrollPosition() - i) > 20;
      this.scrollPosition.update((S) => (r ? i + 1 : a ? S - a : S + 1));
    }
    clickCard(i) {
      if (this.hasDraggedDistance) {
        this.hasDraggedDistance = !1;
        return;
      }
      this.currentIdx.set(i);
      let a = this.getClickedCardReview();
      this.showCompanyInfoDialog.set(a !== null);
    }
    getClickedCardReview() {
      let i = this.members[this.currentIdx()],
        a = wa.find((r) => `${i.name} ${i.surname}` === r.name);
      if (!a) throw new Error("Member not found");
      return a;
    }
    getAllowScroll() {
      let a = this.wrapperRef.nativeElement,
        r = Yt(a, "mouseenter").pipe(ti(() => !1)),
        S = Yt(a, "mouseover").pipe(ti(() => !1)),
        x = Yt(a, "mouseleave").pipe(
          Uo(1333),
          ti(() => !this.isDragging),
        ),
        B = Yt(a, "mousedown").pipe(ti(() => !1)),
        te = Yt(a, "touchstart"),
        Pe = Yt(a, "touchend"),
        et = te.pipe(
          qr(() =>
            Yo(
              jr(!1),
              Pe.pipe(
                Uo(1333),
                Jr(te),
                ti(() => !0),
              ),
            ),
          ),
        );
      return Yo(r, S, x, B, et).pipe(Ur(!0));
    }
    static ɵfac = function (a) {
      return new (a || c)();
    };
    static ɵcmp = J({
      type: c,
      selectors: [["pool-land-our-team"]],
      viewQuery: function (a, r) {
        if ((a & 1 && (gt(wl, 5), gt(Dl, 5)), a & 2)) {
          let S;
          (mt((S = _t())) && (r.wrapperRef = S.first),
            mt((S = _t())) && (r.contentRef = S.first));
        }
      },
      decls: 15,
      vars: 2,
      consts: () => {
        let i;
        i = "Anda dapat mempercayai kami";
        let a;
        return (
          (a =
            " Temui orang-orang" +
            "\uFFFD#5\uFFFD\uFFFD/#5\uFFFD" +
            "di balik " +
            "\uFFFD#6\uFFFD" +
            "POOOL" +
            "\uFFFD/#6\uFFFD" +
            ""),
          [
            ["scrollingWrapper", ""],
            ["scrollingContent", ""],
            i,
            a,
            [
              1,
              "max-block-width",
              "flex",
              "w-full",
              "flex-col",
              "items-center",
              "justify-center",
              "gap-4",
              "py-20",
              "lg:gap-8",
              "lg:py-28",
            ],
            [1, "text-primary-blue", "text-lg", "font-medium"],
            [
              "bubbleAnimation",
              "",
              1,
              "text-center",
              "text-4xl",
              "font-extrabold",
              "uppercase",
              "tracking-tighter",
              "lg:text-5xl",
            ],
            [1, "text-primary-blue", "tracking-[-0.2em]"],
            [1, "no-scrollbar", "w-full"],
            [
              "tabindex",
              "0",
              1,
              "scrolling-wrapper",
              "no-scrollbar",
              3,
              "scrollLeft",
            ],
            [1, "scrolling-content"],
            [
              "id",
              "scrollingContentCard",
              "tabindex",
              "0",
              1,
              "hover:bg-accent-green",
              "group",
              "w-80",
              "cursor-pointer",
              "overflow-hidden",
              "rounded-2xl",
              "bg-[#DBECFF]",
              "p-4",
              "shadow-lg",
            ],
            [
              "tabindex",
              "0",
              1,
              "fixed",
              "left-0",
              "top-0",
              "z-50",
              "flex",
              "h-full",
              "w-full",
              "items-center",
              "justify-center",
              "bg-black/30",
              "p-4",
              "backdrop-blur-md",
              "lg:p-28",
            ],
            [
              "id",
              "scrollingContentCard",
              "tabindex",
              "0",
              1,
              "hover:bg-accent-green",
              "group",
              "w-80",
              "cursor-pointer",
              "overflow-hidden",
              "rounded-2xl",
              "bg-[#DBECFF]",
              "p-4",
              "shadow-lg",
              3,
              "click",
              "keydown.enter",
              "keydown.space",
            ],
            [1, "relative", "overflow-hidden", "rounded-xl"],
            [
              1,
              "h-[360px]",
              "w-full",
              "object-cover",
              "object-center",
              3,
              "src",
              "alt",
              "ngStyle",
            ],
            [1, "absolute", "bottom-0", "w-full", "p-4"],
            [
              1,
              "-gap-1",
              "flex",
              "flex-col",
              "text-lg",
              "font-medium",
              "leading-snug",
            ],
            [1, "member-quote", "relative", "text-lg", "text-white"],
            [
              1,
              "text-primary-blue",
              "group-hover:text-accent-green",
              "absolute",
              "-top-6",
              "font-mono",
              "text-5xl",
              "leading-none",
              "lg:-top-5",
            ],
            [
              1,
              "relative",
              "grid",
              "w-full",
              "grid-cols-[1fr_max-content]",
              "items-start",
              "justify-center",
              "pt-4",
            ],
            [1, "flex", "flex-col", "text-2xl", "font-bold"],
            [
              1,
              "text-primary-blue",
              "col",
              "border-accent-green",
              "absolute",
              "right-1",
              "top-2",
              "mt-2",
              "min-w-max",
              "rounded-full",
              "border",
              "border-solid",
              "bg-white",
              "px-2",
              "py-0.5",
              "text-sm",
            ],
            [
              "tabindex",
              "0",
              1,
              "fixed",
              "left-0",
              "top-0",
              "z-50",
              "flex",
              "h-full",
              "w-full",
              "items-center",
              "justify-center",
              "bg-black/30",
              "p-4",
              "backdrop-blur-md",
              "lg:p-28",
              3,
              "click",
              "keydown.enter",
            ],
            [
              "tabindex",
              "0",
              1,
              "relative",
              "flex",
              "items-start",
              "py-12",
              3,
              "click",
              "keydown.enter",
            ],
            [
              1,
              "no-scrollbar",
              "flex",
              "max-h-[480px]",
              "max-w-[600px]",
              "flex-col",
              "justify-start",
              "gap-4",
              "overflow-x-hidden",
              "overflow-y-scroll",
              "rounded-xl",
              "bg-white",
              "p-8",
              "py-12",
              "lg:max-h-[600px]",
              "lg:p-12",
            ],
            [
              "src",
              "svg/quote-mark.svg",
              "alt",
              "quote-mark",
              1,
              "flex",
              "w-min",
              "xl:h-7",
            ],
            [1, "flex", "flex-col", "gap-1"],
            [1, "text-3xl", "font-medium"],
            [
              1,
              "text-muted",
              "ml-[2px]",
              "text-[12px]",
              "font-medium",
              "leading-none",
            ],
            [1, "text-primary-blue", "font-semibold", "tracking-tighter"],
            [1, "text-sm", 3, "innerHTML"],
            [
              1,
              "pointer-events-none",
              "absolute",
              "bottom-12",
              "left-0",
              "h-20",
              "w-full",
              "rounded-xl",
              "bg-gradient-to-t",
              "from-white",
              "to-transparent",
            ],
            [
              "aria-label",
              "Close",
              "tabindex",
              "0",
              1,
              "absolute",
              "-right-4",
              "top-8",
              "flex",
              "h-10",
              "w-10",
              "items-center",
              "justify-center",
              "rounded-full",
              "bg-white",
              "text-center",
              "text-black",
              "transition-all",
              "hover:bg-gray-200",
              3,
              "click",
              "keydown.enter",
            ],
            [1, "my-auto", "text-2xl"],
          ]
        );
      },
      template: function (a, r) {
        (a & 1 &&
          (l(0, "div", 4)(1, "span", 5),
          Y(2, 2),
          p(),
          l(3, "h3", 6),
          V(4, 3),
          f(5, "br")(6, "span", 7),
          X(),
          p(),
          l(7, "div", 8)(8, "div", 9, 0)(10, "div", 10, 1),
          vt(12, Rl, 19, 12, "div", 11, Nl),
          p()()()(),
          Nt(14, Ll, 16, 3, "div", 12)),
          a & 2 &&
            (F(8),
            M("scrollLeft", r.scrollPosition()),
            F(4),
            yt(r.members),
            F(2),
            tn(r.showCompanyInfoDialog() ? 14 : -1)));
      },
      dependencies: [ie, ra, aa, We],
      styles: [
        "[_nghost-%COMP%]{display:flex;width:100%;align-items:center;justify-content:center}.member-quote[_ngcontent-%COMP%]{text-shadow:0px 2px 3px rgba(0,0,0,.3)}.scrolling-wrapper[_ngcontent-%COMP%]{display:flex;flex-wrap:nowrap;overflow-y:hidden;overflow-x:scroll;position:relative;width:100%}.scrolling-wrapper[_ngcontent-%COMP%]:active{cursor:grabbing!important}.scrolling-wrapper.dragging[_ngcontent-%COMP%]{user-select:none;-webkit-user-select:none;-moz-user-select:none;-ms-user-select:none}.scrolling-content[_ngcontent-%COMP%]{display:flex;gap:1rem;padding:1.5rem .5rem}@keyframes _ngcontent-%COMP%_scroll{0%{transform:translate(0)}to{transform:translate(calc(-100% - 1rem))}}",
      ],
      changeDetection: 0,
    });
  }
  return c;
})();
var kl = (c, E) => ({
  "border-primary-blue bg-secondary-green text-primary-blue border": c,
  "hover:text-primary-blue text-muted": E,
});
function Il(c, E) {
  if (c & 1) {
    let i = Ht();
    (l(0, "button", 15),
      pe("click", function () {
        let r = se(i).$index,
          S = _e();
        return ce((S.activeTab = r));
      }),
      y(1),
      p());
  }
  if (c & 2) {
    let i = E.$implicit,
      a = E.$index,
      r = _e();
    (M("ngClass", En(2, kl, r.activeTab === a, r.activeTab !== a)),
      F(),
      ne(" ", i.tabName, " "));
  }
}
function $l(c, E) {
  if ((c & 1 && (l(0, "h3", 14), f(1, "p", 16), p()), c & 2)) {
    let i = _e();
    (F(), M("innerHTML", i.ownershipTabs[i.activeTab].iconDesc, vn));
  }
}
var Ma = (() => {
  class c {
    activeTab = 0;
    ownershipTabs = [
      {
        tabName: "BPN",
        name: "Kepemilikan properti yang disahkan oleh notaris",
        description:
          "Miliki properti secara langsung dengan sertifikat notaris yang sah dan terdaftar di Badan Pertanahan Nasional (BPN). Kepemilikan properti yang dapat Anda percayai, sepenuhnya aman dan transparan.",
        iconUrl: "/webp/page-block-ownreship/bpn.webp",
        iconDesc: `National Land Agency <br />
        of Indonesia<br />
        (BPN)`,
      },
      {
        tabName: "NIC",
        name: "Kontrak Investasi yang diaktakan oleh Notaris",
        description:
          "Anda sepenuhnya dilindungi oleh hukum Indonesia melalui kontrak investasi ini, yang ditandatangani oleh CEO dan notaris resmi kami, Erma Novita.",
        iconUrl: "/webp/page-block-ownreship/nic.webp",
      },
    ];
    static ɵfac = function (a) {
      return new (a || c)();
    };
    static ɵcmp = J({
      type: c,
      selectors: [["pool-land-ownership"]],
      decls: 19,
      vars: 4,
      consts: () => {
        let i;
        i = "Anda dapat mempercayai kami";
        let a;
        return (
          (a =
            "KEPEMILIKAN DIATUR" +
            "\uFFFD#6\uFFFD\uFFFD/#6\uFFFD" +
            "OLEH PEMERINTAH INDONESIA"),
          [
            i,
            a,
            [
              1,
              "max-block-width",
              "flex",
              "h-max",
              "w-full",
              "flex-col",
              "items-center",
              "justify-center",
              "gap-4",
              "pb-[120px]",
              "pt-20",
              "lg:gap-10",
              "lg:p-20",
              "lg:py-0",
              "lg:pb-[120px]",
              "xl:p-28",
              "xl:py-0",
              "xl:pb-[120px]",
            ],
            [
              1,
              "flex",
              "flex-col",
              "items-center",
              "justify-center",
              "gap-4",
              "text-center",
            ],
            [1, "text-primary-blue", "text-lg", "font-medium"],
            [
              "bubbleAnimation",
              "",
              1,
              "max-w-[800px]",
              "text-center",
              "text-4xl",
              "font-extrabold",
              "uppercase",
              "tracking-tighter",
              "xl:text-5xl",
            ],
            [
              1,
              "border-secondary-green",
              "flex",
              "items-center",
              "rounded-full",
              "border",
              "p-[1px]",
            ],
            [
              1,
              "min-w-max",
              "rounded-full",
              "px-3",
              "py-1",
              "text-center",
              "font-bold",
              "sm:px-6",
              "sm:py-2",
              "xl:text-lg",
              3,
              "ngClass",
            ],
            [
              1,
              "flex",
              "w-full",
              "flex-col-reverse",
              "overflow-hidden",
              "rounded-t-2xl",
              "lg:grid",
              "lg:grid-cols-[750fr_450fr]",
              "lg:rounded-2xl",
            ],
            [
              1,
              "flex",
              "flex-col",
              "gap-4",
              "bg-[#1E1E1E]",
              "p-4",
              "py-8",
              "text-center",
              "text-white",
              "lg:gap-6",
              "lg:px-12",
              "lg:py-20",
              "lg:text-start",
              "xl:px-20",
              "xl:py-28",
            ],
            [
              1,
              "text-accent-green",
              "text-4xl",
              "font-medium",
              "leading-tight",
              "tracking-tighter",
              "lg:text-5xl",
            ],
            [
              1,
              "max-w-md",
              "pb-20",
              "text-base",
              "font-medium",
              "text-gray-300",
              "lg:pb-0",
              "lg:text-lg",
            ],
            [
              1,
              "bg-primary-blue",
              "relative",
              "flex",
              "flex-col",
              "items-center",
              "justify-center",
              "gap-4",
              "bg-[url(/svg/background-lines-top-center.svg)]",
              "bg-cover",
              "bg-center",
              "bg-no-repeat",
              "px-8",
              "py-8",
              "lg:px-20",
              "lg:py-10",
            ],
            ["alt", "National Land Agency logo", 1, "h-44", 3, "src"],
            [
              1,
              "text-center",
              "text-lg",
              "font-medium",
              "tracking-tighter",
              "text-[#FDC32E]",
            ],
            [
              1,
              "min-w-max",
              "rounded-full",
              "px-3",
              "py-1",
              "text-center",
              "font-bold",
              "sm:px-6",
              "sm:py-2",
              "xl:text-lg",
              3,
              "click",
              "ngClass",
            ],
            [3, "innerHTML"],
          ]
        );
      },
      template: function (a, r) {
        (a & 1 &&
          (l(0, "div", 2)(1, "div", 3)(2, "h4", 4),
          Y(3, 0),
          p(),
          l(4, "h3", 5),
          V(5, 1),
          f(6, "br"),
          X(),
          p()(),
          l(7, "div", 6),
          vt(8, Il, 2, 5, "button", 7, yn),
          p(),
          l(10, "div", 8)(11, "div", 9)(12, "h2", 10),
          y(13),
          p(),
          l(14, "p", 11),
          y(15),
          p()(),
          l(16, "div", 12),
          f(17, "img", 13),
          Nt(18, $l, 2, 1, "h3", 14),
          p()()()),
          a & 2 &&
            (F(8),
            yt(r.ownershipTabs),
            F(5),
            ne(" ", r.ownershipTabs[r.activeTab].name, " "),
            F(2),
            ne(" ", r.ownershipTabs[r.activeTab].description, " "),
            F(2),
            M("src", r.ownershipTabs[r.activeTab].iconUrl, Ot),
            F(),
            tn(r.ownershipTabs[r.activeTab].iconDesc ? 18 : -1)));
      },
      dependencies: [ie, li, We],
      encapsulation: 2,
      changeDetection: 0,
    });
  }
  return c;
})();
var ir = (c) => [c, "USD", "symbol", "1.0-0", "en-US"],
  Na = (() => {
    class c {
      locale = De(ai);
      initialInvestment = Hn(500);
      recurrentInvestment = Hn(0);
      totalProfit = oa(() =>
        parseFloat(
          this.calculateCompoundInterest1(
            this.initialInvestment(),
            this.recurrentInvestment(),
            28,
            5,
          ).toFixed(2),
        ),
      );
      get isIndonesian() {
        return this.locale.includes("id");
      }
      calculateCompoundInterest1(i, a, r, S) {
        let x = r / 100,
          B = 12,
          te = S,
          Pe = i * Math.pow(1 + x / B, B * te),
          et = a * ((Math.pow(1 + x / B, B * te) - 1) / (x / B));
        return Pe + et;
      }
      static ɵfac = function (a) {
        return new (a || c)();
      };
      static ɵcmp = J({
        type: c,
        selectors: [["pool-land-grow"]],
        decls: 60,
        vars: 36,
        consts: () => {
          let i;
          i = "Miliki Saham";
          let a;
          a = "Investasi Awal";
          let r;
          r = "Investasi rutin bulanan";
          let S;
          S =
            "Perhitungan dan proyeksi ini didasarkan pada data historis dari REID di Indonesia.";
          let x;
          x =
            "Investasi yang diproyeksikan" +
            "\uFFFD#31\uFFFD\uFFFD/#31\uFFFD" +
            "kembali:";
          let B;
          B = "5 tahun";
          let te;
          te =
            "Pengembalian Sewa" + "\uFFFD#40\uFFFD\uFFFD/#40\uFFFD" + "Tahunan";
          let Pe;
          Pe =
            "Pertumbuhan" + "\uFFFD#44\uFFFD\uFFFD/#44\uFFFD" + "Nilai Tahunan";
          let et;
          return (
            (et =
              "Total Pengembalian" +
              "\uFFFD#48\uFFFD\uFFFD/#48\uFFFD" +
              "Tahunan"),
            [
              a,
              r,
              S,
              x,
              B,
              te,
              Pe,
              et,
              [1, "max-block-width", "h-full", "w-screen", "max-w-[1440px]"],
              [
                1,
                "flex",
                "h-full",
                "w-full",
                "flex-col",
                "items-center",
                "justify-center",
                "gap-5",
                "py-20",
                "lg:gap-10",
              ],
              [
                "bubbleAnimation",
                "",
                "alt",
                "Logo",
                1,
                "h-[30%]",
                "w-[90%]",
                "lg:h-[128px]",
                "lg:w-auto",
                3,
                "src",
              ],
              [
                1,
                "flex",
                "w-full",
                "flex-col",
                "items-center",
                "justify-center",
                "p-4",
                "tracking-tighter",
                "lg:flex-row",
                "lg:px-16",
                "xl:px-44",
              ],
              [1, "flex", "w-full", "lg:justify-end"],
              [1, "grid", "w-full", "grid-rows-[1fr_auto_1fr]"],
              [
                1,
                "-mb-16",
                "flex",
                "flex-col",
                "justify-center",
                "gap-8",
                "rounded-xl",
                "bg-white",
                "p-4",
                "pb-20",
                "text-lg",
                "font-medium",
                "lg:mb-0",
                "lg:rounded-r-none",
                "lg:px-16",
                "lg:py-24",
              ],
              [1, "flex", "w-full", "flex-col", "justify-center"],
              [1, "flex", "items-center", "justify-between", "gap-4"],
              [
                "variant",
                "primary-blue-with-white",
                "size",
                "md",
                3,
                "valueChanged",
                "value",
                "min",
                "max",
                "step",
              ],
              [
                "variant",
                "accent-green-with-white",
                "size",
                "md",
                3,
                "valueChanged",
                "value",
                "min",
                "max",
                "step",
              ],
              [
                1,
                "hidden",
                "p-3",
                "px-3",
                "text-center",
                "text-sm",
                "text-[#CDF6F6]",
                "lg:block",
              ],
              [1, "flex", "w-full", "px-4", "lg:w-auto", "lg:p-0"],
              [
                1,
                "bg-primary-grey",
                "flex",
                "w-full",
                "flex-col",
                "items-center",
                "justify-center",
                "gap-7",
                "rounded-xl",
                "px-4",
                "py-8",
                "lg:mx-0",
                "lg:w-fit",
                "lg:min-w-[396px]",
                "lg:gap-10",
                "lg:px-8",
                "lg:py-12",
              ],
              [
                1,
                "flex",
                "flex-col",
                "items-center",
                "justify-center",
                "gap-2",
                "text-center",
                "sm:gap-4",
              ],
              [1, "text-base", "font-medium", "tracking-tighter", "sm:text-xl"],
              [1, "text-primary-blue", "text-3xl", "font-bold"],
              [1, "text-[40px]", "font-bold", "leading-none", "lg:text-5xl"],
              [
                1,
                "grid",
                "grid-cols-[1fr_max-content_1fr_max-content_1fr]",
                "grid-rows-2",
                "place-items-center",
                "gap-2",
                "text-center",
                "text-2xl",
              ],
              [1, "text-xs", "text-[#80858F]"],
              [1, "text-2xl", "font-medium"],
              ["text", i, "variant", "primary-secondary-green"],
            ]
          );
        },
        template: function (a, r) {
          (a & 1 &&
            (l(0, "div", 8)(1, "div", 9),
            f(2, "img", 10),
            l(3, "div", 11)(4, "div", 12)(5, "div", 13),
            f(6, "div"),
            l(7, "div", 14)(8, "div", 15)(9, "div", 16)(10, "span"),
            Y(11, 0),
            p(),
            l(12, "span"),
            y(13),
            bt(14, "currency"),
            p()(),
            l(15, "pool-land-input-range-slider", 17),
            pe("valueChanged", function (x) {
              return r.initialInvestment.set(x);
            }),
            p()(),
            l(16, "div", 15)(17, "div", 16)(18, "span"),
            Y(19, 1),
            p(),
            l(20, "span"),
            y(21),
            bt(22, "currency"),
            p()(),
            l(23, "pool-land-input-range-slider", 18),
            pe("valueChanged", function (x) {
              return r.recurrentInvestment.set(x);
            }),
            p()()(),
            l(24, "span", 19),
            Y(25, 2),
            p()()(),
            l(26, "div", 20)(27, "div", 21)(28, "p", 22)(29, "span", 23),
            V(30, 3),
            f(31, "br"),
            X(),
            p(),
            l(32, "span", 24),
            Y(33, 4),
            p(),
            l(34, "span", 25),
            y(35),
            bt(36, "currency"),
            p()(),
            l(37, "div", 26)(38, "span", 27),
            V(39, 5),
            f(40, "br"),
            X(),
            p(),
            f(41, "span"),
            l(42, "span", 27),
            V(43, 6),
            f(44, "br"),
            X(),
            p(),
            f(45, "span"),
            l(46, "span", 27),
            V(47, 7),
            f(48, "br"),
            X(),
            p(),
            l(49, "span", 28),
            y(50, "15%"),
            p(),
            l(51, "span"),
            y(52, "+"),
            p(),
            l(53, "span", 28),
            y(54, "13%"),
            p(),
            l(55, "span"),
            y(56, "="),
            p(),
            l(57, "span", 28),
            y(58, "28%"),
            p()(),
            f(59, "pool-land-button", 29),
            p()()()()()),
            a & 2 &&
              (F(2),
              M(
                "src",
                r.isIndonesian
                  ? "/svg/pool-grow-logo-id.svg"
                  : "/svg/pool-grow-logo.svg",
                Ot,
              ),
              F(11),
              Rt(so(14, 12, ri(30, ir, r.initialInvestment()))),
              F(2),
              M("value", r.initialInvestment())("min", 0)("max", 1e6)(
                "step",
                500,
              ),
              F(6),
              Rt(so(22, 18, ri(32, ir, r.recurrentInvestment()))),
              F(2),
              M("value", r.recurrentInvestment())("min", 0)("max", 1e6)(
                "step",
                500,
              ),
              F(12),
              Rt(so(36, 24, ri(34, ir, r.totalProfit())))));
        },
        dependencies: [ie, po, xn, We, Sa],
        styles: [
          "[_nghost-%COMP%]{display:flex;height:100%;width:100%;--tw-bg-opacity: 1;background-color:rgb(0 0 255 / var(--tw-bg-opacity, 1));background-image:url(/svg/background-lines-top-center.svg);background-size:cover;background-clip:content-box;background-position:center;background-repeat:no-repeat}",
        ],
        changeDetection: 0,
      });
    }
    return c;
  })();
var Gl = (c, E) => E.title;
function Bl(c, E) {
  if (
    (c & 1 &&
      (l(0, "div", 1),
      f(1, "img", 2),
      l(2, "div", 3)(3, "h4", 4),
      y(4),
      p()(),
      l(5, "p", 5),
      y(6),
      p()()),
    c & 2)
  ) {
    let i = E.$implicit;
    (F(),
      lo("src", i.icon, Ot),
      F(3),
      ne(" ", i.title, " "),
      F(2),
      ne(" ", i.description, " "));
  }
}
var Ra = (() => {
  class c {
    propertyFeatures = [
      {
        icon: "/png/properties-features/insurance.webp",
        title: "Kepemilikan langsung",
        description:
          "Sertifikat digital Anda adalah bukti kepemilikan resmi yang diakui pemerintah. Tanpa perantara. Langsung milik Anda.",
      },
      {
        icon: "/png/properties-features/best-price.webp",
        title: "Pembayaran bulanan",
        description:
          "Dapatkan penghasilan pasif dari penyewaan harian atau bulanan setiap bulannya dan raih keuntungan dari pertumbuhan nilai jangka panjang yang tinggi.",
      },
      {
        icon: "/png/properties-features/best-price.webp",
        title: "ROI tertinggi",
        description:
          "Bosan dengan manajer properti yang haus keuntungan? Kami menyediakan layanan kami dengan biaya rendah, sehingga Anda dapat mewujudkan ROI tertinggi.",
      },
    ];
    static ɵfac = function (a) {
      return new (a || c)();
    };
    static ɵcmp = J({
      type: c,
      selectors: [["pool-land-properties-features"]],
      decls: 3,
      vars: 0,
      consts: [
        [
          1,
          "grid",
          "grid-cols-2",
          "gap-4",
          "rounded-xl",
          "border",
          "border-[#C0C0C0]",
          "p-4",
          "lg:grid-cols-3",
          "lg:gap-8",
          "lg:border-none",
          "lg:bg-none",
          "lg:p-0",
        ],
        [1, "grid", "grid-rows-[max-content_1fr_auto]", "gap-2"],
        ["alt", "property-insurance", 1, "w-12", 3, "src"],
        [1, "flex", "items-center"],
        [1, "text-lg", "font-medium", "tracking-tight"],
        [1, "text-muted", "text-xs", "tracking-tighter"],
      ],
      template: function (a, r) {
        (a & 1 && (l(0, "div", 0), vt(1, Bl, 7, 3, "div", 1, Gl), p()),
          a & 2 && (F(), yt(r.propertyFeatures)));
      },
      dependencies: [ie],
      encapsulation: 2,
      changeDetection: 0,
    });
  }
  return c;
})();
var La = (() => {
  class c {
    property;
    isHovered = !1;
    static ɵfac = function (a) {
      return new (a || c)();
    };
    static ɵcmp = J({
      type: c,
      selectors: [["pool-land-property-card"]],
      inputs: { property: "property", isHovered: "isHovered" },
      decls: 31,
      vars: 20,
      consts: () => {
        let i;
        i =
          "" +
          "\uFFFD#12\uFFFD" +
          "" +
          "\uFFFD0\uFFFD" +
          "" +
          "\uFFFD/#12\uFFFD" +
          " Investors ";
        let a;
        a = " Average Appreciation ";
        let r;
        r = "Average ROI";
        let S;
        return (
          (S = "" + "\uFFFD0\uFFFD" + " Shares Sold at"),
          [
            i,
            a,
            r,
            S,
            [
              1,
              "property-card",
              "flex",
              "w-[250px]",
              "flex-col",
              "gap-4",
              "overflow-hidden",
              "rounded-xl",
              "bg-white",
              "p-4",
              "lg:w-[372px]",
              "lg:gap-6",
              "lg:px-5",
            ],
            [
              "alt",
              "Property Image",
              1,
              "max-h-[200px]",
              "rounded-lg",
              "object-cover",
              "object-bottom",
              "lg:rounded-xl",
              3,
              "src",
            ],
            [1, "text-xl", "font-medium", "lg:text-2xl"],
            [1, "border", "border-[#F1F1F1]"],
            [1, "flex", "flex-col", "gap-2"],
            [
              1,
              "text-primary-blue",
              "flex",
              "items-center",
              "justify-between",
              "gap-2",
            ],
            [1, "property-card__label", "xl:text-xl"],
            [1, "text-sm", "xl:text-base"],
            [1, "font-bold"],
            [1, "flex", "items-center", "justify-between", "gap-2"],
            [1, "property-card__label", "text-sm", "xl:text-base"],
            [1, "text-sm", "font-bold", "xl:text-base"],
            [1, "flex", "items-center", "justify-center", "gap-2"],
            [
              1,
              "bg-primary-blue",
              "w-full",
              "rounded-full",
              "px-3",
              "py-2",
              "text-base",
              "font-medium",
              "tracking-tighter",
              "text-white",
              "lg:px-5",
              "lg:py-3",
              "lg:text-xl",
            ],
            [1, "flex", "items-center", "justify-center", "gap-4", "xl:gap-8"],
            [1, "min-w-max"],
          ]
        );
      },
      template: function (a, r) {
        (a & 1 &&
          (l(0, "div", 4),
          f(1, "img", 5),
          l(2, "h3", 6),
          y(3),
          p(),
          f(4, "hr", 7),
          l(5, "div", 8)(6, "div", 9)(7, "p", 10),
          y(8),
          bt(9, "currency"),
          p(),
          l(10, "p", 11),
          V(11, 0),
          f(12, "span", 12),
          X(),
          p()(),
          l(13, "div", 13)(14, "p", 14),
          Y(15, 1),
          p(),
          l(16, "p", 15),
          y(17),
          p()(),
          l(18, "div", 13)(19, "p", 14),
          Y(20, 2),
          p(),
          l(21, "p", 15),
          y(22),
          p()()(),
          l(23, "div", 16)(24, "button", 17)(25, "div", 18)(26, "p", 19),
          Y(27, 3),
          p(),
          l(28, "p"),
          y(29),
          bt(30, "currency"),
          p()()()()()),
          a & 2 &&
            (ao("force-hover", r.isHovered),
            F(),
            M("src", r.property.image, Ot),
            F(2),
            Rt(r.property.address),
            F(5),
            ne(" ", tr(9, 10, r.property.price, "USD", "symbol", "1.0-0"), " "),
            F(4),
            qo(r.property.investors),
            Jo(11),
            F(5),
            ne(" ", r.property.averageAppreciation, "% "),
            F(5),
            ne("", r.property.roi, "%"),
            F(5),
            qo(r.property.sharesSold),
            Jo(27),
            F(2),
            ne(
              " ",
              tr(30, 15, r.property.pricePerShare, "USD", "symbol", "1.0-0"),
              " ",
            )));
      },
      dependencies: [ie, po],
      styles: [
        ".property-card__label[_ngcontent-%COMP%]{font-weight:500;--tw-text-opacity: 1;color:rgb(128 133 143 / var(--tw-text-opacity, 1))}.property-card[_ngcontent-%COMP%]:hover, .property-card.force-hover[_ngcontent-%COMP%]{background-color:#98fb96}.property-card[_ngcontent-%COMP%]:hover > hr[_ngcontent-%COMP%], .property-card.force-hover[_ngcontent-%COMP%] > hr[_ngcontent-%COMP%]{--tw-border-opacity: 1;border-color:rgb(0 0 255 / var(--tw-border-opacity, 1))}.property-card[_ngcontent-%COMP%]:hover > h3[_ngcontent-%COMP%], .property-card.force-hover[_ngcontent-%COMP%] > h3[_ngcontent-%COMP%]{--tw-text-opacity: 1;color:rgb(0 0 255 / var(--tw-text-opacity, 1))}.property-card[_ngcontent-%COMP%]:hover   .property-card__label[_ngcontent-%COMP%], .property-card.force-hover[_ngcontent-%COMP%]   .property-card__label[_ngcontent-%COMP%]{--tw-text-opacity: 1;color:rgb(0 0 0 / var(--tw-text-opacity, 1))}",
      ],
      changeDetection: 0,
    });
  }
  return c;
})();
var Hl = () => ({
    address: "Nomad Palm Residence",
    price: 266e4,
    investors: 306,
    averageAppreciation: 10,
    roi: 10,
    sharesSold: 1900,
    pricePerShare: 1400,
    image: "/png/Nomad Palm Residence.webp",
  }),
  zl = () => ({
    address: "Luna Bay Villa",
    price: 494e3,
    investors: 119,
    averageAppreciation: 12,
    roi: 13,
    sharesSold: 380,
    pricePerShare: 1300,
    image: "/png/Luna Bay Villa.webp",
  }),
  Wl = () => ({
    address: "Azure Echo House",
    price: 185e3,
    investors: 48,
    averageAppreciation: 12,
    roi: 12,
    sharesSold: 185,
    pricePerShare: 1e3,
    image: "/png/Azure Echo House.webp",
  }),
  ka = (() => {
    class c {
      static ɵfac = function (a) {
        return new (a || c)();
      };
      static ɵcmp = J({
        type: c,
        selectors: [["pool-land-property-cards"]],
        decls: 25,
        vars: 7,
        consts: () => {
          let i;
          i =
            " tentukan, cintai," +
            "\uFFFD#6\uFFFD\uFFFD/#6\uFFFD" +
            "jadikan milikmu ";
          let a;
          ((a =
            " POOOL memungkinkan Anda untuk berinvestasi di real estat dan bisnis premium mulai dari 150.000 Rp - didukung oleh teknologi mutakhir dan kontrak investasi yang disahkan oleh notaris yang membuat setiap investasi menjadi aman, transparan, dan unik." +
            "[\uFFFD#10\uFFFD\uFFFD/#10\uFFFD|\uFFFD#11\uFFFD\uFFFD/#11\uFFFD]" +
            "" +
            "[\uFFFD#10\uFFFD\uFFFD/#10\uFFFD|\uFFFD#11\uFFFD\uFFFD/#11\uFFFD]" +
            "Apa yang dimulai sebagai tren pasar telah berkembang menjadi platform investasi global, memberikan ribuan pengguna akses ke aset premium berimbal hasil tinggi. "),
            (a = Et(a)));
          let r;
          return (
            (r =
              " Sanggahan: Platform POOOL saat ini sedang dalam tahap pengembangan dan konten ini hanya ditujukan untuk demonstrasi dan presentasi konsep."),
            [
              i,
              a,
              r,
              [
                1,
                "max-block-width",
                "flex",
                "h-max",
                "items-center",
                "py-20",
                "lg:py-28",
              ],
              [
                1,
                "grid",
                "grid-cols-1",
                "place-content-center",
                "gap-6",
                "lg:grid-cols-2",
                "lg:gap-16",
              ],
              [
                1,
                "flex",
                "flex-col",
                "justify-center",
                "gap-6",
                "p-4",
                "lg:pl-16",
                "lg:pr-0",
                "xl:pl-28",
              ],
              ["bubbleAnimation", ""],
              [
                1,
                "text-3xl",
                "font-extrabold",
                "uppercase",
                "tracking-tighter",
                "lg:text-5xl",
              ],
              [1, "flex", "flex-col-reverse", "gap-6", "lg:flex-col"],
              [1, "flex", "flex-col", "gap-6"],
              [1, "flex", "flex-col", "lg:gap-2"],
              [
                "bubbleAnimation",
                "",
                1,
                "property-card-fan",
                "relative",
                "flex",
                "items-center",
                "justify-center",
                "px-4",
                "py-8",
              ],
              [1, "property-card-wrapper", "relative", "h-[500px]", "w-full"],
              [
                1,
                "absolute",
                "left-0",
                "top-1/2",
                "z-10",
                "-translate-y-1/2",
                "-rotate-6",
                "scale-90",
                "transform",
                "overflow-hidden",
                "rounded-xl",
                "opacity-80",
              ],
              [3, "property"],
              [
                1,
                "absolute",
                "left-1/2",
                "top-1/2",
                "z-20",
                "-translate-x-1/2",
                "-translate-y-1/2",
                "scale-110",
                "transform",
                "overflow-hidden",
                "rounded-xl",
              ],
              [3, "property", "isHovered"],
              [
                1,
                "absolute",
                "right-0",
                "top-1/2",
                "z-10",
                "-translate-y-1/2",
                "rotate-6",
                "scale-90",
                "transform",
                "overflow-hidden",
                "rounded-xl",
                "opacity-80",
              ],
              [1, "px-10", "text-center", "text-sm", "text-[#A5A2A2]"],
            ]
          );
        },
        template: function (a, r) {
          (a & 1 &&
            (l(0, "div", 3)(1, "div", 4)(2, "div", 5)(3, "div", 6)(4, "h2", 7),
            V(5, 0),
            f(6, "br"),
            X(),
            p()(),
            l(7, "div", 8)(8, "p"),
            V(9, 1),
            f(10, "br")(11, "br"),
            X(),
            p(),
            l(12, "div", 9),
            f(13, "pool-land-properties-features"),
            p()()(),
            l(14, "div", 10)(15, "div", 11)(16, "div", 12)(17, "div", 13),
            f(18, "pool-land-property-card", 14),
            p(),
            l(19, "div", 15),
            f(20, "pool-land-property-card", 16),
            p(),
            l(21, "div", 17),
            f(22, "pool-land-property-card", 14),
            p()()(),
            l(23, "p", 18),
            Y(24, 2),
            p()()()()),
            a & 2 &&
              (F(18),
              M("property", oi(4, Hl)),
              F(2),
              M("property", oi(5, zl))("isHovered", !0),
              F(2),
              M("property", oi(6, Wl))));
        },
        dependencies: [ie, La, Ra, We],
        styles: [
          "[_nghost-%COMP%]{display:flex;--tw-bg-opacity: 1;background-color:rgb(249 249 249 / var(--tw-bg-opacity, 1))}.scrolling-wrapper[_ngcontent-%COMP%]{display:flex;flex-wrap:nowrap;overflow:hidden;position:relative;width:100%}.scrolling-content[_ngcontent-%COMP%]{display:flex;gap:1rem;padding:1.5rem .5rem;animation:_ngcontent-%COMP%_scroll 40s linear infinite;will-change:transform}.scrolling-wrapper[_ngcontent-%COMP%]:hover   .scrolling-content[_ngcontent-%COMP%]{animation-play-state:paused}@keyframes _ngcontent-%COMP%_scroll{0%{transform:translate(0)}to{transform:translate(calc(-100% - 1rem))}}.property-card-fan[_ngcontent-%COMP%]{perspective:1000px}.property-card-wrapper[_ngcontent-%COMP%]{transition:all .3s ease}.property-card-wrapper[_ngcontent-%COMP%] > div[_ngcontent-%COMP%]{transition:all .3s ease;box-shadow:0 10px 20px #0000001a}.property-card-wrapper[_ngcontent-%COMP%]:hover > div[_ngcontent-%COMP%]{transform-origin:bottom center}.property-card-wrapper[_ngcontent-%COMP%]:hover > div[_ngcontent-%COMP%]:first-child{transform:translateY(-50%) translate(-10px) rotate(-10deg) scale(.95)}.property-card-wrapper[_ngcontent-%COMP%]:hover > div[_ngcontent-%COMP%]:last-child{transform:translateY(-50%) translate(10px) rotate(10deg) scale(.95)}.property-card-wrapper[_ngcontent-%COMP%]:hover > div[_ngcontent-%COMP%]:nth-child(2){transform:translate(-50%) translateY(-50%) scale(1.15);z-index:30}",
        ],
        changeDetection: 0,
      });
    }
    return c;
  })();
function Vl(c, E) {
  if (c & 1) {
    let i = Ht();
    (l(0, "div", 16),
      pe("click", function () {
        se(i);
        let r = _e();
        return ce(r.handleVideoClick(!1));
      }),
      l(1, "div", 17),
      pe("click", function (r) {
        return (se(i), ce(r.stopPropagation()));
      }),
      l(2, "button", 18),
      pe("click", function () {
        se(i);
        let r = _e();
        return ce(r.handleVideoClick(!1));
      }),
      l(3, "span", 19),
      y(4, "\xD7"),
      p()(),
      l(5, "div", 20),
      f(6, "iframe", 21),
      bt(7, "safeUrl"),
      p()()());
  }
  if (c & 2) {
    let i = _e();
    (F(6), M("src", zn(7, 1, i.videoSrc), ro));
  }
}
var Ia = (() => {
  class c {
    cd = De(bn);
    videoSrc =
      "https://www.youtube.com/embed/bp9WoCmwU6M?si=hdRWxDBJJQeFqohh?autoplay=1";
    showVideo = !1;
    handleVideoClick(i) {
      ((this.showVideo = i), this.cd.detectChanges());
    }
    static ɵfac = function (a) {
      return new (a || c)();
    };
    static ɵcmp = J({
      type: c,
      selectors: [["pool-land-visual-story"]],
      decls: 16,
      vars: 1,
      consts: () => {
        let i;
        i = "Tonton video";
        let a;
        a =
          " Perhatikan cara kerja " +
          "\uFFFD#5\uFFFD\uFFFD/#5\uFFFD" +
          "" +
          "\uFFFD#6\uFFFD" +
          "POOOL" +
          "\uFFFD/#6\uFFFD" +
          "";
        let r;
        r =
          " Kisah visual singkat tentang cara mudah dan cepat berinvestasi di real estat dan menghasilkan keuntungan langsung ";
        let S;
        return (
          (S =
            " Perhatikan betapa mudahnya membeli dan menjual saham properti dengan " +
            "\uFFFD#14\uFFFD" +
            "POOOL" +
            "\uFFFD/#14\uFFFD" +
            ""),
          [
            a,
            r,
            S,
            [
              1,
              "max-block-width",
              "flex",
              "w-screen",
              "flex-col",
              "items-center",
              "gap-5",
              "p-6",
              "py-32",
              "lg:px-[190px]",
              "lg:py-28",
            ],
            [
              1,
              "relative",
              "flex",
              "h-[340px]",
              "w-[340px]",
              "items-center",
              "justify-center",
              "rounded-xl",
              "bg-[url(/webp/watch-video.webp)]",
              "bg-cover",
              "bg-center",
              "bg-no-repeat",
              "text-center",
              "text-white",
              "shadow-2xl",
              "lg:h-[572px]",
              "lg:w-full",
            ],
            [
              1,
              "flex",
              "h-full",
              "w-full",
              "flex-col",
              "items-center",
              "justify-center",
              "gap-4",
              "rounded-xl",
              "bg-black",
              "bg-opacity-50",
              "p-4",
              "lg:gap-6",
              "lg:p-8",
            ],
            [
              "bubbleAnimation",
              "",
              1,
              "text-2xl",
              "font-medium",
              "tracking-tighter",
              "lg:text-5xl",
            ],
            [1, "block", "lg:hidden"],
            [1, "text-secondary-green"],
            [1, "max-w-[520px]", "text-base", "lg:text-lg"],
            ["text", i, "variant", "tertiary", 3, "click"],
            [
              "src",
              "/webp/video-overlay-property-card.webp",
              "alt",
              "property card overlay",
              1,
              "absolute",
              "bottom-[82%]",
              "right-[4%]",
              "w-[128px]",
              "lg:-right-[10%]",
              "lg:bottom-[5%]",
              "lg:transform",
              "lg:[rotate:6deg]",
              "xl:w-[200px]",
            ],
            [
              "src",
              "/svg/video-overlay-card.svg",
              "alt",
              "all time returns card overlay",
              1,
              "absolute",
              "left-[25%]",
              "top-[88%]",
              "w-[170px]",
              "lg:-left-[10%]",
              "lg:top-[5%]",
              "lg:w-[250px]",
            ],
            [1, "text-muted", "text-sm", "lg:text-lg"],
            [
              1,
              "text-primary-blue",
              "font-semibold",
              "uppercase",
              "tracking-wide",
            ],
            [
              "class",
              "fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-80 p-4",
              3,
              "click",
              4,
              "ngIf",
            ],
            [
              1,
              "fixed",
              "inset-0",
              "z-50",
              "flex",
              "items-center",
              "justify-center",
              "bg-black",
              "bg-opacity-80",
              "p-4",
              3,
              "click",
            ],
            [
              1,
              "relative",
              "w-full",
              "max-w-4xl",
              "rounded-xl",
              "bg-black",
              "shadow-2xl",
              3,
              "click",
            ],
            [
              "aria-label",
              "Close video",
              1,
              "absolute",
              "-right-4",
              "-top-4",
              "flex",
              "h-10",
              "w-10",
              "items-center",
              "justify-center",
              "rounded-full",
              "bg-white",
              "text-black",
              "transition-all",
              "hover:bg-gray-200",
              3,
              "click",
            ],
            [1, "text-2xl"],
            [1, "aspect-video", "w-full", "overflow-hidden", "rounded-xl"],
            [
              "width",
              "100%",
              "height",
              "100%",
              "frameborder",
              "0",
              "allow",
              "accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture",
              "allowfullscreen",
              "",
              3,
              "src",
            ],
          ]
        );
      },
      template: function (a, r) {
        (a & 1 &&
          (l(0, "div", 3)(1, "div", 4)(2, "div", 5)(3, "h3", 6),
          V(4, 0),
          f(5, "br", 7)(6, "span", 8),
          X(),
          p(),
          l(7, "p", 9),
          Y(8, 1),
          p(),
          l(9, "pool-land-button", 10),
          pe("click", function () {
            return r.handleVideoClick(!0);
          }),
          p()(),
          f(10, "img", 11)(11, "img", 12),
          p(),
          l(12, "p", 13),
          V(13, 2),
          f(14, "span", 14),
          X(),
          p()(),
          Nt(15, Vl, 8, 3, "div", 15)),
          a & 2 && (F(15), M("ngIf", r.showVideo)));
      },
      dependencies: [ie, co, _o, xn, We],
      styles: [
        "[_nghost-%COMP%]{display:flex;width:100%;align-items:center;justify-content:center;background-image:url(/svg/background-buy-shares.svg);background-size:cover;background-position:center;background-repeat:no-repeat}",
      ],
    });
  }
  return c;
})();
var Xl = ["video"];
function jl(c, E) {
  if (
    (c & 1 &&
      (l(0, "div", 47),
      f(1, "img", 48),
      l(2, "span", 49),
      y(3),
      p()(),
      l(4, "div", 50),
      f(5, "img", 51),
      l(6, "span", 52),
      y(7),
      p()(),
      f(8, "div", 53)),
    c & 2)
  ) {
    let i = E.$implicit;
    (F(),
      lo("src", i.imageUrl, Ot),
      F(2),
      Rt(i.name),
      F(4),
      ne(" ", i.averageRentalROI, " "));
  }
}
var $a = (() => {
  class c {
    video;
    cities = [
      {
        name: "Bali",
        averageRentalROI: "7%\u201315%",
        imageUrl: "svg/world-map/bali.svg",
      },
      {
        name: "Phuket",
        averageRentalROI: "9%\u201310%",
        imageUrl: "svg/world-map/thailand.svg",
      },
      {
        name: "Dubai",
        averageRentalROI: "5%\u20139%",
        imageUrl: "svg/world-map/oae.svg",
      },
      {
        name: "Miami",
        averageRentalROI: "7%",
        imageUrl: "svg/world-map/usa.svg",
      },
      {
        name: "German",
        averageRentalROI: "2%",
        imageUrl: "svg/world-map/de.svg",
      },
      {
        name: "London",
        averageRentalROI: "4.4%",
        imageUrl: "svg/world-map/gb.svg",
      },
      {
        name: "Tokyo",
        averageRentalROI: "3%\u20135%",
        imageUrl: "svg/world-map/japan.svg",
      },
    ];
    ngAfterViewInit() {
      ((this.video.nativeElement.muted = !0), this.video.nativeElement.play());
    }
    static ɵfac = function (a) {
      return new (a || c)();
    };
    static ɵcmp = J({
      type: c,
      selectors: [["pool-land-world-map"]],
      viewQuery: function (a, r) {
        if ((a & 1 && gt(Xl, 5), a & 2)) {
          let S;
          mt((S = _t())) && (r.video = S.first);
        }
      },
      decls: 66,
      vars: 0,
      consts: () => {
        let i;
        i = "Pendapatan di seluruh dunia";
        let a;
        a = "PASAR " + "\uFFFD#8\uFFFD" + "TOP 1" + "\uFFFD/#8\uFFFD" + "";
        let r;
        r = "Lokasi";
        let S;
        S = "Rata-rata ROI Sewa";
        let x;
        ((x =
          "" +
          "\uFFFD#21\uFFFD" +
          "Mengapa " +
          "\uFFFD#22\uFFFD" +
          "Bali?" +
          "[\uFFFD/#22\uFFFD|\uFFFD/#21\uFFFD|\uFFFD/#25\uFFFD|\uFFFD/#26\uFFFD|\uFFFD/#29\uFFFD|\uFFFD/#30\uFFFD]" +
          "" +
          "[\uFFFD/#22\uFFFD|\uFFFD/#21\uFFFD|\uFFFD/#25\uFFFD|\uFFFD/#26\uFFFD|\uFFFD/#29\uFFFD|\uFFFD/#30\uFFFD]" +
          "" +
          "[\uFFFD#23\uFFFD\uFFFD/#23\uFFFD|\uFFFD#24\uFFFD\uFFFD/#24\uFFFD|\uFFFD#27\uFFFD\uFFFD/#27\uFFFD|\uFFFD#28\uFFFD\uFFFD/#28\uFFFD]" +
          "" +
          "[\uFFFD#23\uFFFD\uFFFD/#23\uFFFD|\uFFFD#24\uFFFD\uFFFD/#24\uFFFD|\uFFFD#27\uFFFD\uFFFD/#27\uFFFD|\uFFFD#28\uFFFD\uFFFD/#28\uFFFD]" +
          "Berinvestasi di pasar real estat Bali yang sedang berkembang pesat dengan " +
          "[\uFFFD#25\uFFFD|\uFFFD#26\uFFFD]" +
          "ambang batas masuk yang rendah, permintaan yang didorong oleh pariwisata yang sangat stabil" +
          "[\uFFFD/#22\uFFFD|\uFFFD/#21\uFFFD|\uFFFD/#25\uFFFD|\uFFFD/#26\uFFFD|\uFFFD/#29\uFFFD|\uFFFD/#30\uFFFD]" +
          ", dan " +
          "[\uFFFD#25\uFFFD|\uFFFD#26\uFFFD]" +
          "hasil sewa tertinggi" +
          "[\uFFFD/#22\uFFFD|\uFFFD/#21\uFFFD|\uFFFD/#25\uFFFD|\uFFFD/#26\uFFFD|\uFFFD/#29\uFFFD|\uFFFD/#30\uFFFD]" +
          " - semua tanpa perlu membeli seluruh vila." +
          "[\uFFFD#23\uFFFD\uFFFD/#23\uFFFD|\uFFFD#24\uFFFD\uFFFD/#24\uFFFD|\uFFFD#27\uFFFD\uFFFD/#27\uFFFD|\uFFFD#28\uFFFD\uFFFD/#28\uFFFD]" +
          "" +
          "[\uFFFD#23\uFFFD\uFFFD/#23\uFFFD|\uFFFD#24\uFFFD\uFFFD/#24\uFFFD|\uFFFD#27\uFFFD\uFFFD/#27\uFFFD|\uFFFD#28\uFFFD\uFFFD/#28\uFFFD]" +
          "Dengan " +
          "[\uFFFD#29\uFFFD|\uFFFD#30\uFFFD]" +
          "POOOL" +
          "[\uFFFD/#22\uFFFD|\uFFFD/#21\uFFFD|\uFFFD/#25\uFFFD|\uFFFD/#26\uFFFD|\uFFFD/#29\uFFFD|\uFFFD/#30\uFFFD]" +
          ", Anda mendapatkan akses ke properti premium di Bali melalui " +
          "[\uFFFD#29\uFFFD|\uFFFD#30\uFFFD]" +
          "kepemilikan fraksional" +
          "[\uFFFD/#22\uFFFD|\uFFFD/#21\uFFFD|\uFFFD/#25\uFFFD|\uFFFD/#26\uFFFD|\uFFFD/#29\uFFFD|\uFFFD/#30\uFFFD]" +
          " - fleksibel, cerdas, dan terkelola sepenuhnya."),
          (x = Et(x)));
        let B;
        B = "hingga";
        let te;
        te = "Pertumbuhan Nilai Tahunan Properti";
        let Pe;
        Pe = "hingga";
        let et;
        et = "Pengembalian Sewa Tahunan";
        let de;
        de = "Sumber Data Pasar";
        let xt;
        return (
          (xt =
            "Bali bukan hanya surga - Bali memberikan " +
            "\uFFFD#64\uFFFD\uFFFD/#64\uFFFD" +
            "" +
            "\uFFFD#65\uFFFD" +
            "ROI tertinggi di seluruh dunia." +
            "\uFFFD/#65\uFFFD" +
            ""),
          [
            ["video", ""],
            i,
            a,
            r,
            S,
            x,
            B,
            te,
            Pe,
            et,
            de,
            xt,
            [
              1,
              "lg:p-30",
              "bg-[#F6FAFF]",
              "p-4",
              "py-20",
              "lg:bg-white",
              "lg:p-28",
            ],
            [
              1,
              "flex",
              "w-full",
              "flex-col",
              "items-center",
              "justify-center",
              "rounded-2xl",
              "lg:border",
              "lg:border-[#F1F1F1]",
              "lg:bg-[#F6FAFF]",
              "lg:p-20",
            ],
            [
              1,
              "grid",
              "w-full",
              "grid-cols-1",
              "gap-9",
              "lg:grid-cols-[1fr_382px]",
              "lg:items-center",
              "lg:gap-16",
            ],
            [
              1,
              "text-primary-blue",
              "-mb-14",
              "text-center",
              "text-lg",
              "font-semibold",
              "lg:col-span-2",
              "lg:text-start",
            ],
            [1, "flex", "h-full", "flex-col"],
            [
              "bubbleAnimation",
              "",
              1,
              "text-center",
              "text-4xl",
              "font-bold",
              "uppercase",
              "tracking-tighter",
              "lg:text-start",
              "lg:text-5xl",
            ],
            [1, "text-primary-blue", "inline-block"],
            [1, "flex", "h-full", "w-full", "items-center", "py-8"],
            [
              "autoplay",
              "",
              "loop",
              "",
              "muted",
              "",
              "playsinline",
              "",
              "width",
              "100%",
              "height",
              "100%",
              "src",
              "webm/webm_globe.webm",
              1,
              "max-w-[428px]",
            ],
            [
              1,
              "grid",
              "grid-cols-[1fr_max-content]",
              "rounded-xl",
              "bg-[#2B32F926]",
              "bg-opacity-15",
            ],
            [
              1,
              "text-muted",
              "rounded-tl-xl",
              "bg-white",
              "px-6",
              "py-3",
              "text-center",
              "text-[12px]",
              "font-semibold",
            ],
            [
              1,
              "text-muted",
              "rounded-tr-xl",
              "bg-white",
              "px-6",
              "py-3",
              "text-center",
              "text-[12px]",
              "font-semibold",
            ],
            [1, "max-w-[520px]", "pt-10", "lg:pt-0"],
            [
              "bubbleAnimation",
              "",
              1,
              "inline-block",
              "text-4xl",
              "font-extrabold",
              "uppercase",
              "tracking-tighter",
              "lg:text-[46px]",
              "lg:leading-[1]",
            ],
            [1, "text-primary-blue"],
            [1, "font-bold"],
            [1, "text-primary-blue", "font-bold"],
            [
              1,
              "row-span-2",
              "flex",
              "h-full",
              "w-[340px]",
              "flex-col",
              "gap-6",
              "lg:w-[382px]",
            ],
            [
              1,
              "hover:bg-secondary-green",
              "bg-secondary-green",
              "grid",
              "grid-cols-[1fr_max-content]",
              "gap-y-6",
              "rounded-xl",
              "p-4",
              "py-5",
            ],
            [1, "col-span-2", "flex"],
            ["iconUrl", "svg/world-map/icon-home.svg"],
            [1, "flex", "flex-col"],
            [1, "text-sm", "font-semibold"],
            [
              "src",
              "svg/world-map/arrow-jagged-up.svg",
              "alt",
              "Arrow up",
              1,
              "inline-flex",
              "pl-1",
            ],
            [1, "text-primary-blue", "text-5xl", "font-extrabold", "uppercase"],
            [1, "text-muted", "text-sm"],
            [1, "flex", "h-full", "items-end"],
            ["src", "svg/world-map/chart-1.svg", "alt", "Chart", 1, "w-[96px]"],
            [
              1,
              "hover:bg-secondary-green",
              "grid",
              "grid-cols-[1fr_max-content]",
              "gap-y-6",
              "rounded-xl",
              "bg-white",
              "p-4",
              "py-5",
            ],
            [1, "col-span-2"],
            ["iconUrl", "svg/world-map/arrow-jagged-up-accent-green.svg"],
            [
              "src",
              "svg/world-map/arrow-jagged-up.svg",
              "alt",
              "Jagged arrow",
              1,
              "inline-flex",
              "pl-1",
            ],
            ["src", "svg/world-map/chart-2.svg", "alt", "Chart", 1, "w-[96px]"],
            [
              "href",
              "https://www.realinfo.id/2024-annual",
              "target",
              "_blank",
              1,
              "text-primary-blue",
              "cursor-pointer",
              "underline",
            ],
            [
              1,
              "max-w-[540px]",
              "text-4xl",
              "font-bold",
              "uppercase",
              "tracking-tighter",
              "lg:-mt-10",
              "lg:text-[46px]",
              "lg:leading-[1]",
            ],
            [
              1,
              "flex",
              "items-center",
              "justify-start",
              "gap-2",
              "py-4",
              "pl-8",
            ],
            ["width", "40px", "alt", "Flag", 3, "src"],
            [1, "text-sm", "font-medium"],
            [
              1,
              "hover:bg-primary-blue",
              "group",
              "mx-auto",
              "flex",
              "h-max",
              "w-max",
              "items-center",
              "justify-center",
              "self-center",
              "rounded-xl",
              "border",
              "border-solid",
              "border-[#17B26A]",
              "bg-[#ABEFC6]",
              "p-1",
              "px-2",
            ],
            ["src", "svg/world-map/arrow-up.svg", "alt", "Arrow Up", 1, "pr-1"],
            [
              1,
              "group-hover:text-accent-green",
              "text-[12px]",
              "font-medium",
              "text-green-700",
            ],
            [1, "col-span-2", "h-[1px]", "w-full", "bg-white"],
          ]
        );
      },
      template: function (a, r) {
        (a & 1 &&
          (l(0, "div", 12)(1, "div", 13)(2, "div", 14)(3, "h5", 15),
          Y(4, 1),
          p(),
          l(5, "div", 16)(6, "h4", 17),
          V(7, 2),
          f(8, "span", 18),
          X(),
          p(),
          l(9, "div", 19),
          f(10, "video", 20, 0),
          p()(),
          l(12, "div", 21)(13, "div", 22),
          Y(14, 3),
          p(),
          l(15, "div", 23),
          Y(16, 4),
          p(),
          vt(17, jl, 9, 3, null, null, na),
          p(),
          l(19, "p", 24),
          V(20, 5),
          l(21, "span", 25),
          f(22, "span", 26),
          p(),
          f(23, "br")(24, "br")(25, "span", 27)(26, "span", 27)(27, "br")(
            28,
            "br",
          )(29, "span", 28)(30, "span", 28),
          X(),
          p(),
          l(31, "div", 29)(32, "div", 30)(33, "div", 31),
          f(34, "pool-land-gradient-icon", 32),
          p(),
          l(35, "div", 33)(36, "div", 34),
          ni(37),
          Y(38, 6),
          ii(),
          f(39, "img", 35),
          p(),
          l(40, "span", 36),
          y(41, "+13%"),
          p(),
          l(42, "span", 37),
          Y(43, 7),
          p()(),
          l(44, "div", 38),
          f(45, "img", 39),
          p()(),
          l(46, "div", 40)(47, "div", 41),
          f(48, "pool-land-gradient-icon", 42),
          p(),
          l(49, "div", 33)(50, "div", 34),
          ni(51),
          Y(52, 8),
          ii(),
          f(53, "img", 43),
          p(),
          l(54, "span", 36),
          y(55, "+15%"),
          p(),
          l(56, "span", 37),
          Y(57, 9),
          p()(),
          l(58, "div", 38),
          f(59, "img", 44),
          p()(),
          l(60, "a", 45),
          Y(61, 10),
          p()(),
          l(62, "p", 46),
          V(63, 11),
          f(64, "br")(65, "span", 26),
          X(),
          p()()()()),
          a & 2 && (F(17), yt(r.cities)));
      },
      dependencies: [ie, Pa, We],
      styles: [
        "[_nghost-%COMP%]{width:100%;display:flex;justify-content:center;align-items:center}",
      ],
      changeDetection: 0,
    });
  }
  return c;
})();
var Ga = (() => {
  class c {
    navbarLinks = [
      { label: "Mengapa kami", anchor: "why-us" },
      { label: "Bagaimana Cara Kerja", anchor: "how-it-works" },
      { label: "Pasar", anchor: "market" },
      { label: "FAQ", anchor: "faq" },
    ];
    faqs = [
      {
        question: "Apakah saya benar-benar memiliki sesuatu?",
        answer:
          "Ya \u2014 dan ini terdaftar secara resmi. Saat Anda berinvestasi melalui POOOL, Anda menerima sertifikat kepemilikan yang dilegalisir sesuai hukum Indonesia. Sertifikat ini terdaftar di Badan Pertanahan Nasional (BPN), yang mengonfirmasi bagian kepemilikan Anda secara hukum atas properti yang memiliki sertifikat resmi.",
        expanded: !1,
      },
      {
        question: "Bisakah saya berinvestasi dari jarak jauh?",
        answer:
          "Ya, Anda tidak perlu tinggal di Indonesia. Kementerian Pekerjaan Umum dan Perumahan Rakyat (PUPR) secara resmi mengizinkan orang asing untuk memiliki kepemilikan properti secara fraksional melalui sertifikat digital. Semua proses dilakukan secara legal dan jarak jauh.",
        expanded: !1,
      },
      {
        question: "Bagaimana saya mendapatkan penghasilan?",
        answer:
          "Anda menerima pendapatan sewa bulanan sesuai bagian kepemilikan Anda. Pembayaran dihitung berdasarkan penghasilan setiap properti dan ditransfer langsung ke rekening bank atau dompet digital Anda, dengan rincian lengkap tersedia di dasbor Anda.",
        expanded: !1,
      },
      {
        question: "Bagaimana cara keluar dari investasi?",
        answer:
          "Anda tidak terikat selamanya. Anda dapat menjual bagian kepemilikan Anda kapan saja melalui platform penjualan kembali internal kami (segera diluncurkan), keluar bersama proyek, atau meminta pembelian kembali oleh perusahaan jika diperlukan. Ini fleksibel dan dirancang untuk kebebasan investor.",
        expanded: !1,
      },
      {
        question: "Apakah ini legal dan diatur oleh hukum?",
        answer:
          "Ya \u2014 POOOL beroperasi dengan kepatuhan penuh di Indonesia. Kami terdaftar sebagai PT. POOOL INTERNATIONAL GROUP dan memiliki sertifikat resmi dari PUPR. Kontrak kami disusun oleh notaris bersertifikat dan ditinjau berdasarkan Undang-Undang Indonesia No. 11/2020 tentang infrastruktur digital.",
        expanded: !1,
      },
      {
        question: "Bagaimana jika properti tidak menghasilkan?",
        answer:
          "Bagian kepemilikan Anda atas properti tetap menjadi milik Anda. Semua properti telah diseleksi, diasuransikan, dan dikelola secara profesional \u2014 namun bahkan dalam masa sulit sekalipun, kepemilikan Anda tidak akan terpengaruh atau hilang.",
        expanded: !1,
      },
      {
        question: "Bagaimana dengan pajak?",
        answer:
          "Kami menyediakan laporan pajak lengkap setiap tahun. Pajak sewa di Indonesia ditangani dalam struktur investasi. Dalam banyak kasus, Anda hanya perlu melaporkan pendapatan di negara asal Anda. Kami sarankan untuk berkonsultasi dengan penasihat pajak lokal Anda untuk kepastian.",
        expanded: !1,
      },
    ];
    static ɵfac = function (a) {
      return new (a || c)();
    };
    static ɵcmp = J({
      type: c,
      selectors: [["pool-land-home"]],
      decls: 33,
      vars: 5,
      consts: () => {
        let i;
        return (
          (i = "Masuk"),
          [
            [3, "navbarLinks"],
            [
              "ngProjectAs",
              "pool-hero-button",
              5,
              ["pool-hero-button"],
              1,
              "flex",
              "items-center",
              "gap-8",
            ],
            ["variant", "primary"],
            [
              "href",
              "/auth/login",
              "target",
              "_blank",
              "rel",
              "noopener noreferrer",
            ],
            ["text", i, "variant", "primary-secondary-green", 3, "noBorder"],
            [
              1,
              "grid-rows-[repeat(auto-fill,",
              "max-content)]",
              "grid",
              "grid-cols-1",
              "overflow-hidden",
            ],
            ["id", "why-us"],
            ["id", "solution", 1, "h-max", "min-h-screen", "pt-20", "lg:pt-32"],
            ["id", "visual-story"],
            ["id", "how-it-works"],
            [1, "min-h-fit"],
            ["id", "market", 1, "flex", "h-max", "min-h-fit"],
            [1, "flex", "h-max", "min-h-fit"],
            ["id", "faq", 1, "flex", "h-max"],
            [3, "faqs"],
          ]
        );
      },
      template: function (a, r) {
        (a & 1 &&
          (f(0, "pool-land-burger-nav", 0),
          l(1, "pool-land-sticky-header", 0)(2, "div", 1),
          f(3, "pool-land-language-selector", 2),
          l(4, "a", 3),
          f(5, "pool-land-button", 4),
          p()()(),
          l(6, "div", 5),
          f(7, "pool-land-home-hero", 0),
          l(8, "section"),
          f(9, "pool-land-join-community"),
          p(),
          l(10, "section", 6),
          f(11, "pool-land-property-cards"),
          p(),
          l(12, "section", 7),
          f(13, "pool-land-investment-amount"),
          p(),
          l(14, "section", 8),
          f(15, "pool-land-visual-story"),
          p(),
          l(16, "section"),
          f(17, "pool-land-closer-look", 9),
          p(),
          l(18, "section", 10),
          f(19, "pool-land-grow"),
          p(),
          l(20, "section", 11),
          f(21, "pool-land-world-map"),
          p(),
          l(22, "section"),
          f(23, "pool-land-ownership"),
          p(),
          l(24, "section"),
          f(25, "pool-land-buy-shares-testimonials"),
          p(),
          l(26, "section"),
          f(27, "pool-land-core-investors"),
          p(),
          l(28, "section", 12),
          f(29, "pool-land-our-team"),
          p(),
          l(30, "section", 13),
          f(31, "pool-land-faq", 14),
          p(),
          f(32, "pool-land-pool-company-info"),
          p()),
          a & 2 &&
            (M("navbarLinks", r.navbarLinks),
            F(),
            M("navbarLinks", r.navbarLinks),
            F(4),
            M("noBorder", !0),
            F(2),
            M("navbarLinks", r.navbarLinks),
            F(24),
            M("faqs", r.faqs)));
      },
      dependencies: [
        ka,
        Aa,
        ba,
        Na,
        $a,
        Ia,
        Ma,
        va,
        Oa,
        Da,
        Ca,
        Ta,
        fa,
        xn,
        _a,
        go,
        ha,
        xa,
      ],
      encapsulation: 2,
    });
  }
  return c;
})();
var Ba = [
  { path: "", component: Ga },
  {
    path: "privacy-policy",
    loadComponent: () =>
      import("./chunk-6AHRJQ47.js").then((c) => c.PrivacyPolicyPageComponent),
  },
  {
    path: "terms-and-conditions",
    loadComponent: () =>
      import("./chunk-7KJW3IND.js").then(
        (c) => c.TermsAndConditionsPageComponent,
      ),
  },
  {
    path: "cookies",
    loadComponent: () =>
      import("./chunk-DI7OJB5C.js").then((c) => c.CookiesPageComponent),
  },
  {
    path: "buy-shares",
    loadComponent: () =>
      import("./chunk-BEB7B7HN.js").then((c) => c.BuySharesPageComponent),
  },
  {
    path: "currency-policy",
    loadComponent: () =>
      import("./chunk-M6GF6SKE.js").then((c) => c.CurrencyPolicyPageComponent),
  },
  {
    path: "**",
    loadComponent: () =>
      import("./chunk-DH2FNK2K.js").then((c) => c.PageNotFoundPageComponent),
  },
];
var Ha = {
  providers: [
    ia({ eventCoalescing: !0 }),
    pa(
      Ba,
      ua({ skipInitialTransition: !0 }),
      da({ anchorScrolling: "enabled", scrollPositionRestoration: "enabled" }),
    ),
    la(),
  ],
};
sa(ya, Ha).catch((c) =>
  console.error(c),
); /**i18n:e2f94bf06bdfc8c8ab493a12299261c375fc525ae09e041ca331cb13279050ab*/
