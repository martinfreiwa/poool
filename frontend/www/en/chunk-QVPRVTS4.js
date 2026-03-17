import "./chunk-ZBVOF6Q3.js";
function Dn(o, e) {
  for (var n = 0; n < e.length; n++) {
    var t = e[n];
    ((t.enumerable = t.enumerable || !1),
      (t.configurable = !0),
      "value" in t && (t.writable = !0),
      Object.defineProperty(o, t.key, t));
  }
}
function xi(o, e, n) {
  return (e && Dn(o.prototype, e), n && Dn(o, n), o);
}
var ve,
  Yr,
  yi,
  Ge,
  Dt,
  Rt,
  Qt,
  On,
  Xt,
  pr,
  An,
  xt,
  ot,
  Yn,
  Fn = function () {
    return (
      ve ||
      (typeof window < "u" && (ve = window.gsap) && ve.registerPlugin && ve)
    );
  },
  Ln = 1,
  Jt = [],
  k = [],
  st = [],
  dr = Date.now,
  on = function (e, n) {
    return n;
  },
  bi = function () {
    var e = pr.core,
      n = e.bridge || {},
      t = e._scrollers,
      r = e._proxies;
    (t.push.apply(t, k),
      r.push.apply(r, st),
      (k = t),
      (st = r),
      (on = function (u, l) {
        return n[u](l);
      }));
  },
  bt = function (e, n) {
    return ~st.indexOf(e) && st[st.indexOf(e) + 1][n];
  },
  gr = function (e) {
    return !!~An.indexOf(e);
  },
  Ae = function (e, n, t, r, i) {
    return e.addEventListener(n, t, { passive: r !== !1, capture: !!i });
  },
  Oe = function (e, n, t, r) {
    return e.removeEventListener(n, t, !!r);
  },
  Or = "scrollLeft",
  Ar = "scrollTop",
  sn = function () {
    return (xt && xt.isPressed) || k.cache++;
  },
  Fr = function (e, n) {
    var t = function r(i) {
      if (i || i === 0) {
        Ln && (Ge.history.scrollRestoration = "manual");
        var u = xt && xt.isPressed;
        ((i = r.v = Math.round(i) || (xt && xt.iOS ? 1 : 0)),
          e(i),
          (r.cacheID = k.cache),
          u && on("ss", i));
      } else
        (n || k.cache !== r.cacheID || on("ref")) &&
          ((r.cacheID = k.cache), (r.v = e()));
      return r.v + r.offset;
    };
    return ((t.offset = 0), e && t);
  },
  we = {
    s: Or,
    p: "left",
    p2: "Left",
    os: "right",
    os2: "Right",
    d: "width",
    d2: "Width",
    a: "x",
    sc: Fr(function (o) {
      return arguments.length
        ? Ge.scrollTo(o, re.sc())
        : Ge.pageXOffset || Dt[Or] || Rt[Or] || Qt[Or] || 0;
    }),
  },
  re = {
    s: Ar,
    p: "top",
    p2: "Top",
    os: "bottom",
    os2: "Bottom",
    d: "height",
    d2: "Height",
    a: "y",
    op: we,
    sc: Fr(function (o) {
      return arguments.length
        ? Ge.scrollTo(we.sc(), o)
        : Ge.pageYOffset || Dt[Ar] || Rt[Ar] || Qt[Ar] || 0;
    }),
  },
  Ye = function (e, n) {
    return (
      ((n && n._ctx && n._ctx.selector) || ve.utils.toArray)(e)[0] ||
      (typeof e == "string" && ve.config().nullTargetWarn !== !1
        ? console.warn("Element not found:", e)
        : null)
    );
  },
  yt = function (e, n) {
    var t = n.s,
      r = n.sc;
    gr(e) && (e = Dt.scrollingElement || Rt);
    var i = k.indexOf(e),
      u = r === re.sc ? 1 : 2;
    (!~i && (i = k.push(e) - 1), k[i + u] || Ae(e, "scroll", sn));
    var l = k[i + u],
      p =
        l ||
        (k[i + u] =
          Fr(bt(e, t), !0) ||
          (gr(e)
            ? r
            : Fr(function (x) {
                return arguments.length ? (e[t] = x) : e[t];
              })));
    return (
      (p.target = e),
      l || (p.smooth = ve.getProperty(e, "scrollBehavior") === "smooth"),
      p
    );
  },
  Lr = function (e, n, t) {
    var r = e,
      i = e,
      u = dr(),
      l = u,
      p = n || 50,
      x = Math.max(500, p * 3),
      Y = function (v, U) {
        var X = dr();
        U || X - u > p
          ? ((i = r), (r = v), (l = u), (u = X))
          : t
            ? (r += v)
            : (r = i + ((v - i) / (X - l)) * (u - l));
      },
      M = function () {
        ((i = r = t ? 0 : r), (l = u = 0));
      },
      h = function (v) {
        var U = l,
          X = i,
          ie = dr();
        return (
          (v || v === 0) && v !== r && Y(v),
          u === l || ie - l > x
            ? 0
            : ((r + (t ? X : -X)) / ((t ? ie : u) - U)) * 1e3
        );
      };
    return { update: Y, reset: M, getVelocity: h };
  },
  fr = function (e, n) {
    return (
      n && !e._gsapAllow && e.preventDefault(),
      e.changedTouches ? e.changedTouches[0] : e
    );
  },
  Rn = function (e) {
    var n = Math.max.apply(Math, e),
      t = Math.min.apply(Math, e);
    return Math.abs(n) >= Math.abs(t) ? n : t;
  },
  zn = function () {
    ((pr = ve.core.globals().ScrollTrigger), pr && pr.core && bi());
  },
  In = function (e) {
    return (
      (ve = e || Fn()),
      !Yr &&
        ve &&
        typeof document < "u" &&
        document.body &&
        ((Ge = window),
        (Dt = document),
        (Rt = Dt.documentElement),
        (Qt = Dt.body),
        (An = [Ge, Dt, Rt, Qt]),
        (yi = ve.utils.clamp),
        (Yn = ve.core.context || function () {}),
        (Xt = "onpointerenter" in Qt ? "pointer" : "mouse"),
        (On = q.isTouch =
          Ge.matchMedia &&
          Ge.matchMedia("(hover: none), (pointer: coarse)").matches
            ? 1
            : "ontouchstart" in Ge ||
                navigator.maxTouchPoints > 0 ||
                navigator.msMaxTouchPoints > 0
              ? 2
              : 0),
        (ot = q.eventTypes =
          (
            "ontouchstart" in Rt
              ? "touchstart,touchmove,touchcancel,touchend"
              : "onpointerdown" in Rt
                ? "pointerdown,pointermove,pointercancel,pointerup"
                : "mousedown,mousemove,mouseup,mouseup"
          ).split(",")),
        setTimeout(function () {
          return (Ln = 0);
        }, 500),
        zn(),
        (Yr = 1)),
      Yr
    );
  };
we.op = re;
k.cache = 0;
var q = (function () {
  function o(n) {
    this.init(n);
  }
  var e = o.prototype;
  return (
    (e.init = function (t) {
      (Yr || In(ve) || console.warn("Please gsap.registerPlugin(Observer)"),
        pr || zn());
      var r = t.tolerance,
        i = t.dragMinimum,
        u = t.type,
        l = t.target,
        p = t.lineHeight,
        x = t.debounce,
        Y = t.preventDefault,
        M = t.onStop,
        h = t.onStopDelay,
        c = t.ignore,
        v = t.wheelSpeed,
        U = t.event,
        X = t.onDragStart,
        ie = t.onDragEnd,
        W = t.onDrag,
        ge = t.onPress,
        T = t.onRelease,
        Ke = t.onRight,
        N = t.onLeft,
        b = t.onUp,
        ke = t.onDown,
        Ie = t.onChangeX,
        g = t.onChangeY,
        ue = t.onChange,
        y = t.onToggleX,
        pt = t.onToggleY,
        oe = t.onHover,
        Pe = t.onHoverEnd,
        Ee = t.onMove,
        z = t.ignoreCheck,
        Q = t.isNormalizer,
        j = t.onGestureStart,
        s = t.onGestureEnd,
        se = t.onWheel,
        At = t.onEnable,
        St = t.onDisable,
        Ze = t.onClick,
        dt = t.scrollSpeed,
        Me = t.capture,
        ee = t.allowClicks,
        De = t.lockAxis,
        me = t.onLockAxis;
      ((this.target = l = Ye(l) || Rt),
        (this.vars = t),
        c && (c = ve.utils.toArray(c)),
        (r = r || 1e-9),
        (i = i || 0),
        (v = v || 1),
        (dt = dt || 1),
        (u = u || "wheel,touch,pointer"),
        (x = x !== !1),
        p || (p = parseFloat(Ge.getComputedStyle(Qt).lineHeight) || 22));
      var Tt,
        Re,
        Xe,
        A,
        K,
        Be,
        Ne,
        a = this,
        He = 0,
        gt = 0,
        kt = t.passive || (!Y && t.passive !== !1),
        Z = yt(l, we),
        ht = yt(l, re),
        Pt = Z(),
        Yt = ht(),
        ce =
          ~u.indexOf("touch") &&
          !~u.indexOf("pointer") &&
          ot[0] === "pointerdown",
        Et = gr(l),
        $ = l.ownerDocument || Dt,
        et = [0, 0, 0],
        $e = [0, 0, 0],
        _t = 0,
        sr = function () {
          return (_t = dr());
        },
        te = function (m, F) {
          return (
            ((a.event = m) && c && ~c.indexOf(m.target)) ||
            (F && ce && m.pointerType !== "touch") ||
            (z && z(m, F))
          );
        },
        Mr = function () {
          (a._vx.reset(), a._vy.reset(), Re.pause(), M && M(a));
        },
        vt = function () {
          var m = (a.deltaX = Rn(et)),
            F = (a.deltaY = Rn($e)),
            f = Math.abs(m) >= r,
            w = Math.abs(F) >= r;
          (ue && (f || w) && ue(a, m, F, et, $e),
            f &&
              (Ke && a.deltaX > 0 && Ke(a),
              N && a.deltaX < 0 && N(a),
              Ie && Ie(a),
              y && a.deltaX < 0 != He < 0 && y(a),
              (He = a.deltaX),
              (et[0] = et[1] = et[2] = 0)),
            w &&
              (ke && a.deltaY > 0 && ke(a),
              b && a.deltaY < 0 && b(a),
              g && g(a),
              pt && a.deltaY < 0 != gt < 0 && pt(a),
              (gt = a.deltaY),
              ($e[0] = $e[1] = $e[2] = 0)),
            (A || Xe) &&
              (Ee && Ee(a),
              Xe && (X && Xe === 1 && X(a), W && W(a), (Xe = 0)),
              (A = !1)),
            Be && !(Be = !1) && me && me(a),
            K && (se(a), (K = !1)),
            (Tt = 0));
        },
        Kt = function (m, F, f) {
          ((et[f] += m),
            ($e[f] += F),
            a._vx.update(m),
            a._vy.update(F),
            x ? Tt || (Tt = requestAnimationFrame(vt)) : vt());
        },
        Zt = function (m, F) {
          (De &&
            !Ne &&
            ((a.axis = Ne = Math.abs(m) > Math.abs(F) ? "x" : "y"), (Be = !0)),
            Ne !== "y" && ((et[2] += m), a._vx.update(m, !0)),
            Ne !== "x" && (($e[2] += F), a._vy.update(F, !0)),
            x ? Tt || (Tt = requestAnimationFrame(vt)) : vt());
        },
        Mt = function (m) {
          if (!te(m, 1)) {
            m = fr(m, Y);
            var F = m.clientX,
              f = m.clientY,
              w = F - a.x,
              _ = f - a.y,
              C = a.isDragging;
            ((a.x = F),
              (a.y = f),
              (C ||
                ((w || _) &&
                  (Math.abs(a.startX - F) >= i ||
                    Math.abs(a.startY - f) >= i))) &&
                ((Xe = C ? 2 : 1), C || (a.isDragging = !0), Zt(w, _)));
          }
        },
        Ft = (a.onPress = function (S) {
          te(S, 1) ||
            (S && S.button) ||
            ((a.axis = Ne = null),
            Re.pause(),
            (a.isPressed = !0),
            (S = fr(S)),
            (He = gt = 0),
            (a.startX = a.x = S.clientX),
            (a.startY = a.y = S.clientY),
            a._vx.reset(),
            a._vy.reset(),
            Ae(Q ? l : $, ot[1], Mt, kt, !0),
            (a.deltaX = a.deltaY = 0),
            ge && ge(a));
        }),
        D = (a.onRelease = function (S) {
          if (!te(S, 1)) {
            Oe(Q ? l : $, ot[1], Mt, !0);
            var m = !isNaN(a.y - a.startY),
              F = a.isDragging,
              f =
                F &&
                (Math.abs(a.x - a.startX) > 3 || Math.abs(a.y - a.startY) > 3),
              w = fr(S);
            (!f &&
              m &&
              (a._vx.reset(),
              a._vy.reset(),
              Y &&
                ee &&
                ve.delayedCall(0.08, function () {
                  if (dr() - _t > 300 && !S.defaultPrevented) {
                    if (S.target.click) S.target.click();
                    else if ($.createEvent) {
                      var _ = $.createEvent("MouseEvents");
                      (_.initMouseEvent(
                        "click",
                        !0,
                        !0,
                        Ge,
                        1,
                        w.screenX,
                        w.screenY,
                        w.clientX,
                        w.clientY,
                        !1,
                        !1,
                        !1,
                        !1,
                        0,
                        null,
                      ),
                        S.target.dispatchEvent(_));
                    }
                  }
                })),
              (a.isDragging = a.isGesturing = a.isPressed = !1),
              M && F && !Q && Re.restart(!0),
              Xe && vt(),
              ie && F && ie(a),
              T && T(a, f));
          }
        }),
        Lt = function (m) {
          return (
            m.touches &&
            m.touches.length > 1 &&
            (a.isGesturing = !0) &&
            j(m, a.isDragging)
          );
        },
        tt = function () {
          return (a.isGesturing = !1) || s(a);
        },
        rt = function (m) {
          if (!te(m)) {
            var F = Z(),
              f = ht();
            (Kt((F - Pt) * dt, (f - Yt) * dt, 1),
              (Pt = F),
              (Yt = f),
              M && Re.restart(!0));
          }
        },
        nt = function (m) {
          if (!te(m)) {
            ((m = fr(m, Y)), se && (K = !0));
            var F =
              (m.deltaMode === 1 ? p : m.deltaMode === 2 ? Ge.innerHeight : 1) *
              v;
            (Kt(m.deltaX * F, m.deltaY * F, 0), M && !Q && Re.restart(!0));
          }
        },
        zt = function (m) {
          if (!te(m)) {
            var F = m.clientX,
              f = m.clientY,
              w = F - a.x,
              _ = f - a.y;
            ((a.x = F),
              (a.y = f),
              (A = !0),
              M && Re.restart(!0),
              (w || _) && Zt(w, _));
          }
        },
        $t = function (m) {
          ((a.event = m), oe(a));
        },
        mt = function (m) {
          ((a.event = m), Pe(a));
        },
        lr = function (m) {
          return te(m) || (fr(m, Y) && Ze(a));
        };
      ((Re = a._dc = ve.delayedCall(h || 0.25, Mr).pause()),
        (a.deltaX = a.deltaY = 0),
        (a._vx = Lr(0, 50, !0)),
        (a._vy = Lr(0, 50, !0)),
        (a.scrollX = Z),
        (a.scrollY = ht),
        (a.isDragging = a.isGesturing = a.isPressed = !1),
        Yn(this),
        (a.enable = function (S) {
          return (
            a.isEnabled ||
              (Ae(Et ? $ : l, "scroll", sn),
              u.indexOf("scroll") >= 0 && Ae(Et ? $ : l, "scroll", rt, kt, Me),
              u.indexOf("wheel") >= 0 && Ae(l, "wheel", nt, kt, Me),
              ((u.indexOf("touch") >= 0 && On) || u.indexOf("pointer") >= 0) &&
                (Ae(l, ot[0], Ft, kt, Me),
                Ae($, ot[2], D),
                Ae($, ot[3], D),
                ee && Ae(l, "click", sr, !0, !0),
                Ze && Ae(l, "click", lr),
                j && Ae($, "gesturestart", Lt),
                s && Ae($, "gestureend", tt),
                oe && Ae(l, Xt + "enter", $t),
                Pe && Ae(l, Xt + "leave", mt),
                Ee && Ae(l, Xt + "move", zt)),
              (a.isEnabled = !0),
              (a.isDragging = a.isGesturing = a.isPressed = A = Xe = !1),
              a._vx.reset(),
              a._vy.reset(),
              (Pt = Z()),
              (Yt = ht()),
              S && S.type && Ft(S),
              At && At(a)),
            a
          );
        }),
        (a.disable = function () {
          a.isEnabled &&
            (Jt.filter(function (S) {
              return S !== a && gr(S.target);
            }).length || Oe(Et ? $ : l, "scroll", sn),
            a.isPressed &&
              (a._vx.reset(), a._vy.reset(), Oe(Q ? l : $, ot[1], Mt, !0)),
            Oe(Et ? $ : l, "scroll", rt, Me),
            Oe(l, "wheel", nt, Me),
            Oe(l, ot[0], Ft, Me),
            Oe($, ot[2], D),
            Oe($, ot[3], D),
            Oe(l, "click", sr, !0),
            Oe(l, "click", lr),
            Oe($, "gesturestart", Lt),
            Oe($, "gestureend", tt),
            Oe(l, Xt + "enter", $t),
            Oe(l, Xt + "leave", mt),
            Oe(l, Xt + "move", zt),
            (a.isEnabled = a.isPressed = a.isDragging = !1),
            St && St(a));
        }),
        (a.kill = a.revert =
          function () {
            a.disable();
            var S = Jt.indexOf(a);
            (S >= 0 && Jt.splice(S, 1), xt === a && (xt = 0));
          }),
        Jt.push(a),
        Q && gr(l) && (xt = a),
        a.enable(U));
    }),
    xi(o, [
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
    o
  );
})();
q.version = "3.12.7";
q.create = function (o) {
  return new q(o);
};
q.register = In;
q.getAll = function () {
  return Jt.slice();
};
q.getById = function (o) {
  return Jt.filter(function (e) {
    return e.vars.id === o;
  })[0];
};
Fn() && ve.registerPlugin(q);
var d,
  tr,
  E,
  B,
  qe,
  L,
  bn,
  Qr,
  Pr,
  br,
  _r,
  zr,
  Ce,
  rn,
  gn,
  Le,
  Xn,
  Bn,
  rr,
  ri,
  ln,
  ni,
  Fe,
  hn,
  ii,
  oi,
  Ot,
  _n,
  wn,
  nr,
  Cn,
  jr,
  vn,
  an,
  Ir = 1,
  Se = Date.now,
  un = Se(),
  je = 0,
  vr = 0,
  Nn = function (e, n, t) {
    var r = Ve(e) && (e.substr(0, 6) === "clamp(" || e.indexOf("max") > -1);
    return ((t["_" + n + "Clamp"] = r), r ? e.substr(6, e.length - 7) : e);
  },
  Hn = function (e, n) {
    return n && (!Ve(e) || e.substr(0, 6) !== "clamp(")
      ? "clamp(" + e + ")"
      : e;
  },
  wi = function o() {
    return vr && requestAnimationFrame(o);
  },
  Wn = function () {
    return (rn = 1);
  },
  Gn = function () {
    return (rn = 0);
  },
  ct = function (e) {
    return e;
  },
  mr = function (e) {
    return Math.round(e * 1e5) / 1e5 || 0;
  },
  si = function () {
    return typeof window < "u";
  },
  li = function () {
    return d || (si() && (d = window.gsap) && d.registerPlugin && d);
  },
  Ut = function (e) {
    return !!~bn.indexOf(e);
  },
  ai = function (e) {
    return (
      (e === "Height" ? Cn : E["inner" + e]) ||
      qe["client" + e] ||
      L["client" + e]
    );
  },
  ui = function (e) {
    return (
      bt(e, "getBoundingClientRect") ||
      (Ut(e)
        ? function () {
            return ((Jr.width = E.innerWidth), (Jr.height = Cn), Jr);
          }
        : function () {
            return wt(e);
          })
    );
  },
  Ci = function (e, n, t) {
    var r = t.d,
      i = t.d2,
      u = t.a;
    return (u = bt(e, "getBoundingClientRect"))
      ? function () {
          return u()[r];
        }
      : function () {
          return (n ? ai(i) : e["client" + i]) || 0;
        };
  },
  Si = function (e, n) {
    return !n || ~st.indexOf(e)
      ? ui(e)
      : function () {
          return Jr;
        };
  },
  ft = function (e, n) {
    var t = n.s,
      r = n.d2,
      i = n.d,
      u = n.a;
    return Math.max(
      0,
      (t = "scroll" + r) && (u = bt(e, t))
        ? u() - ui(e)()[i]
        : Ut(e)
          ? (qe[t] || L[t]) - ai(r)
          : e[t] - e["offset" + r],
    );
  },
  Xr = function (e, n) {
    for (var t = 0; t < rr.length; t += 3)
      (!n || ~n.indexOf(rr[t + 1])) && e(rr[t], rr[t + 1], rr[t + 2]);
  },
  Ve = function (e) {
    return typeof e == "string";
  },
  Te = function (e) {
    return typeof e == "function";
  },
  xr = function (e) {
    return typeof e == "number";
  },
  Bt = function (e) {
    return typeof e == "object";
  },
  hr = function (e, n, t) {
    return e && e.progress(n ? 0 : 1) && t && e.pause();
  },
  cn = function (e, n) {
    if (e.enabled) {
      var t = e._ctx
        ? e._ctx.add(function () {
            return n(e);
          })
        : n(e);
      t && t.totalTime && (e.callbackAnimation = t);
    }
  },
  jt = Math.abs,
  ci = "left",
  fi = "top",
  Sn = "right",
  Tn = "bottom",
  Ht = "width",
  Wt = "height",
  wr = "Right",
  Cr = "Left",
  Sr = "Top",
  Tr = "Bottom",
  ne = "padding",
  Je = "margin",
  or = "Width",
  kn = "Height",
  ae = "px",
  Qe = function (e) {
    return E.getComputedStyle(e);
  },
  Ti = function (e) {
    var n = Qe(e).position;
    e.style.position = n === "absolute" || n === "fixed" ? n : "relative";
  },
  Un = function (e, n) {
    for (var t in n) t in e || (e[t] = n[t]);
    return e;
  },
  wt = function (e, n) {
    var t =
        n &&
        Qe(e)[gn] !== "matrix(1, 0, 0, 1, 0, 0)" &&
        d
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
      r = e.getBoundingClientRect();
    return (t && t.progress(0).kill(), r);
  },
  en = function (e, n) {
    var t = n.d2;
    return e["offset" + t] || e["client" + t] || 0;
  },
  pi = function (e) {
    var n = [],
      t = e.labels,
      r = e.duration(),
      i;
    for (i in t) n.push(t[i] / r);
    return n;
  },
  ki = function (e) {
    return function (n) {
      return d.utils.snap(pi(e), n);
    };
  },
  Pn = function (e) {
    var n = d.utils.snap(e),
      t =
        Array.isArray(e) &&
        e.slice(0).sort(function (r, i) {
          return r - i;
        });
    return t
      ? function (r, i, u) {
          u === void 0 && (u = 0.001);
          var l;
          if (!i) return n(r);
          if (i > 0) {
            for (r -= u, l = 0; l < t.length; l++) if (t[l] >= r) return t[l];
            return t[l - 1];
          } else for (l = t.length, r += u; l--; ) if (t[l] <= r) return t[l];
          return t[0];
        }
      : function (r, i, u) {
          u === void 0 && (u = 0.001);
          var l = n(r);
          return !i || Math.abs(l - r) < u || l - r < 0 == i < 0
            ? l
            : n(i < 0 ? r - e : r + e);
        };
  },
  Pi = function (e) {
    return function (n, t) {
      return Pn(pi(e))(n, t.direction);
    };
  },
  Br = function (e, n, t, r) {
    return t.split(",").forEach(function (i) {
      return e(n, i, r);
    });
  },
  de = function (e, n, t, r, i) {
    return e.addEventListener(n, t, { passive: !r, capture: !!i });
  },
  pe = function (e, n, t, r) {
    return e.removeEventListener(n, t, !!r);
  },
  Nr = function (e, n, t) {
    ((t = t && t.wheelHandler), t && (e(n, "wheel", t), e(n, "touchmove", t)));
  },
  Vn = {
    startColor: "green",
    endColor: "red",
    indent: 0,
    fontSize: "16px",
    fontWeight: "normal",
  },
  Hr = { toggleActions: "play", anticipatePin: 0 },
  tn = { top: 0, left: 0, center: 0.5, bottom: 1, right: 1 },
  qr = function (e, n) {
    if (Ve(e)) {
      var t = e.indexOf("="),
        r = ~t ? +(e.charAt(t - 1) + 1) * parseFloat(e.substr(t + 1)) : 0;
      (~t && (e.indexOf("%") > t && (r *= n / 100), (e = e.substr(0, t - 1))),
        (e =
          r +
          (e in tn
            ? tn[e] * n
            : ~e.indexOf("%")
              ? (parseFloat(e) * n) / 100
              : parseFloat(e) || 0)));
    }
    return e;
  },
  Wr = function (e, n, t, r, i, u, l, p) {
    var x = i.startColor,
      Y = i.endColor,
      M = i.fontSize,
      h = i.indent,
      c = i.fontWeight,
      v = B.createElement("div"),
      U = Ut(t) || bt(t, "pinType") === "fixed",
      X = e.indexOf("scroller") !== -1,
      ie = U ? L : t,
      W = e.indexOf("start") !== -1,
      ge = W ? x : Y,
      T =
        "border-color:" +
        ge +
        ";font-size:" +
        M +
        ";color:" +
        ge +
        ";font-weight:" +
        c +
        ";pointer-events:none;white-space:nowrap;font-family:sans-serif,Arial;z-index:1000;padding:4px 8px;border-width:0;border-style:solid;";
    return (
      (T += "position:" + ((X || p) && U ? "fixed;" : "absolute;")),
      (X || p || !U) &&
        (T += (r === re ? Sn : Tn) + ":" + (u + parseFloat(h)) + "px;"),
      l &&
        (T +=
          "box-sizing:border-box;text-align:left;width:" +
          l.offsetWidth +
          "px;"),
      (v._isStart = W),
      v.setAttribute("class", "gsap-marker-" + e + (n ? " marker-" + n : "")),
      (v.style.cssText = T),
      (v.innerText = n || n === 0 ? e + "-" + n : e),
      ie.children[0] ? ie.insertBefore(v, ie.children[0]) : ie.appendChild(v),
      (v._offset = v["offset" + r.op.d2]),
      Kr(v, 0, r, W),
      v
    );
  },
  Kr = function (e, n, t, r) {
    var i = { display: "block" },
      u = t[r ? "os2" : "p2"],
      l = t[r ? "p2" : "os2"];
    ((e._isFlipped = r),
      (i[t.a + "Percent"] = r ? -100 : 0),
      (i[t.a] = r ? "1px" : 0),
      (i["border" + u + or] = 1),
      (i["border" + l + or] = 0),
      (i[t.p] = n + "px"),
      d.set(e, i));
  },
  P = [],
  mn = {},
  Er,
  qn = function () {
    return Se() - je > 34 && (Er || (Er = requestAnimationFrame(Ct)));
  },
  er = function () {
    (!Fe || !Fe.isPressed || Fe.startX > L.clientWidth) &&
      (k.cache++,
      Fe ? Er || (Er = requestAnimationFrame(Ct)) : Ct(),
      je || qt("scrollStart"),
      (je = Se()));
  },
  fn = function () {
    ((oi = E.innerWidth), (ii = E.innerHeight));
  },
  yr = function (e) {
    (k.cache++,
      (e === !0 ||
        (!Ce &&
          !ni &&
          !B.fullscreenElement &&
          !B.webkitFullscreenElement &&
          (!hn ||
            oi !== E.innerWidth ||
            Math.abs(E.innerHeight - ii) > E.innerHeight * 0.25))) &&
        Qr.restart(!0));
  },
  Vt = {},
  Ei = [],
  di = function o() {
    return pe(R, "scrollEnd", o) || Nt(!0);
  },
  qt = function (e) {
    return (
      (Vt[e] &&
        Vt[e].map(function (n) {
          return n();
        })) ||
      Ei
    );
  },
  Ue = [],
  gi = function (e) {
    for (var n = 0; n < Ue.length; n += 5)
      (!e || (Ue[n + 4] && Ue[n + 4].query === e)) &&
        ((Ue[n].style.cssText = Ue[n + 1]),
        Ue[n].getBBox && Ue[n].setAttribute("transform", Ue[n + 2] || ""),
        (Ue[n + 3].uncache = 1));
  },
  En = function (e, n) {
    var t;
    for (Le = 0; Le < P.length; Le++)
      ((t = P[Le]),
        t && (!n || t._ctx === n) && (e ? t.kill(1) : t.revert(!0, !0)));
    ((jr = !0), n && gi(n), n || qt("revert"));
  },
  hi = function (e, n) {
    (k.cache++,
      (n || !ze) &&
        k.forEach(function (t) {
          return Te(t) && t.cacheID++ && (t.rec = 0);
        }),
      Ve(e) && (E.history.scrollRestoration = wn = e));
  },
  ze,
  Gt = 0,
  Kn,
  Mi = function () {
    if (Kn !== Gt) {
      var e = (Kn = Gt);
      requestAnimationFrame(function () {
        return e === Gt && Nt(!0);
      });
    }
  },
  _i = function () {
    (L.appendChild(nr),
      (Cn = (!Fe && nr.offsetHeight) || E.innerHeight),
      L.removeChild(nr));
  },
  Zn = function (e) {
    return Pr(
      ".gsap-marker-start, .gsap-marker-end, .gsap-marker-scroller-start, .gsap-marker-scroller-end",
    ).forEach(function (n) {
      return (n.style.display = e ? "none" : "block");
    });
  },
  Nt = function (e, n) {
    if (
      ((qe = B.documentElement),
      (L = B.body),
      (bn = [E, B, qe, L]),
      je && !e && !jr)
    ) {
      de(R, "scrollEnd", di);
      return;
    }
    (_i(),
      (ze = R.isRefreshing = !0),
      k.forEach(function (r) {
        return Te(r) && ++r.cacheID && (r.rec = r());
      }));
    var t = qt("refreshInit");
    (ri && R.sort(),
      n || En(),
      k.forEach(function (r) {
        Te(r) && (r.smooth && (r.target.style.scrollBehavior = "auto"), r(0));
      }),
      P.slice(0).forEach(function (r) {
        return r.refresh();
      }),
      (jr = !1),
      P.forEach(function (r) {
        if (r._subPinOffset && r.pin) {
          var i = r.vars.horizontal ? "offsetWidth" : "offsetHeight",
            u = r.pin[i];
          (r.revert(!0, 1), r.adjustPinSpacing(r.pin[i] - u), r.refresh());
        }
      }),
      (vn = 1),
      Zn(!0),
      P.forEach(function (r) {
        var i = ft(r.scroller, r._dir),
          u = r.vars.end === "max" || (r._endClamp && r.end > i),
          l = r._startClamp && r.start >= i;
        (u || l) &&
          r.setPositions(
            l ? i - 1 : r.start,
            u ? Math.max(l ? i : r.start + 1, i) : r.end,
            !0,
          );
      }),
      Zn(!1),
      (vn = 0),
      t.forEach(function (r) {
        return r && r.render && r.render(-1);
      }),
      k.forEach(function (r) {
        Te(r) &&
          (r.smooth &&
            requestAnimationFrame(function () {
              return (r.target.style.scrollBehavior = "smooth");
            }),
          r.rec && r(r.rec));
      }),
      hi(wn, 1),
      Qr.pause(),
      Gt++,
      (ze = 2),
      Ct(2),
      P.forEach(function (r) {
        return Te(r.vars.onRefresh) && r.vars.onRefresh(r);
      }),
      (ze = R.isRefreshing = !1),
      qt("refresh"));
  },
  xn = 0,
  Zr = 1,
  kr,
  Ct = function (e) {
    if (e === 2 || (!ze && !jr)) {
      ((R.isUpdating = !0), kr && kr.update(0));
      var n = P.length,
        t = Se(),
        r = t - un >= 50,
        i = n && P[0].scroll();
      if (
        ((Zr = xn > i ? -1 : 1),
        ze || (xn = i),
        r &&
          (je && !rn && t - je > 200 && ((je = 0), qt("scrollEnd")),
          (_r = un),
          (un = t)),
        Zr < 0)
      ) {
        for (Le = n; Le-- > 0; ) P[Le] && P[Le].update(0, r);
        Zr = 1;
      } else for (Le = 0; Le < n; Le++) P[Le] && P[Le].update(0, r);
      R.isUpdating = !1;
    }
    Er = 0;
  },
  yn = [
    ci,
    fi,
    Tn,
    Sn,
    Je + Tr,
    Je + wr,
    Je + Sr,
    Je + Cr,
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
  $r = yn.concat([
    Ht,
    Wt,
    "boxSizing",
    "max" + or,
    "max" + kn,
    "position",
    Je,
    ne,
    ne + Sr,
    ne + wr,
    ne + Tr,
    ne + Cr,
  ]),
  Di = function (e, n, t) {
    ir(t);
    var r = e._gsap;
    if (r.spacerIsNative) ir(r.spacerState);
    else if (e._gsap.swappedIn) {
      var i = n.parentNode;
      i && (i.insertBefore(e, n), i.removeChild(n));
    }
    e._gsap.swappedIn = !1;
  },
  pn = function (e, n, t, r) {
    if (!e._gsap.swappedIn) {
      for (var i = yn.length, u = n.style, l = e.style, p; i--; )
        ((p = yn[i]), (u[p] = t[p]));
      ((u.position = t.position === "absolute" ? "absolute" : "relative"),
        t.display === "inline" && (u.display = "inline-block"),
        (l[Tn] = l[Sn] = "auto"),
        (u.flexBasis = t.flexBasis || "auto"),
        (u.overflow = "visible"),
        (u.boxSizing = "border-box"),
        (u[Ht] = en(e, we) + ae),
        (u[Wt] = en(e, re) + ae),
        (u[ne] = l[Je] = l[fi] = l[ci] = "0"),
        ir(r),
        (l[Ht] = l["max" + or] = t[Ht]),
        (l[Wt] = l["max" + kn] = t[Wt]),
        (l[ne] = t[ne]),
        e.parentNode !== n &&
          (e.parentNode.insertBefore(n, e), n.appendChild(e)),
        (e._gsap.swappedIn = !0));
    }
  },
  Ri = /([A-Z])/g,
  ir = function (e) {
    if (e) {
      var n = e.t.style,
        t = e.length,
        r = 0,
        i,
        u;
      for ((e.t._gsap || d.core.getCache(e.t)).uncache = 1; r < t; r += 2)
        ((u = e[r + 1]),
          (i = e[r]),
          u
            ? (n[i] = u)
            : n[i] && n.removeProperty(i.replace(Ri, "-$1").toLowerCase()));
    }
  },
  Gr = function (e) {
    for (var n = $r.length, t = e.style, r = [], i = 0; i < n; i++)
      r.push($r[i], t[$r[i]]);
    return ((r.t = e), r);
  },
  Oi = function (e, n, t) {
    for (var r = [], i = e.length, u = t ? 8 : 0, l; u < i; u += 2)
      ((l = e[u]), r.push(l, l in n ? n[l] : e[u + 1]));
    return ((r.t = e.t), r);
  },
  Jr = { left: 0, top: 0 },
  $n = function (e, n, t, r, i, u, l, p, x, Y, M, h, c, v) {
    (Te(e) && (e = e(p)),
      Ve(e) &&
        e.substr(0, 3) === "max" &&
        (e = h + (e.charAt(4) === "=" ? qr("0" + e.substr(3), t) : 0)));
    var U = c ? c.time() : 0,
      X,
      ie,
      W;
    if ((c && c.seek(0), isNaN(e) || (e = +e), xr(e)))
      (c &&
        (e = d.utils.mapRange(
          c.scrollTrigger.start,
          c.scrollTrigger.end,
          0,
          h,
          e,
        )),
        l && Kr(l, t, r, !0));
    else {
      Te(n) && (n = n(p));
      var ge = (e || "0").split(" "),
        T,
        Ke,
        N,
        b;
      ((W = Ye(n, p) || L),
        (T = wt(W) || {}),
        (!T || (!T.left && !T.top)) &&
          Qe(W).display === "none" &&
          ((b = W.style.display),
          (W.style.display = "block"),
          (T = wt(W)),
          b ? (W.style.display = b) : W.style.removeProperty("display")),
        (Ke = qr(ge[0], T[r.d])),
        (N = qr(ge[1] || "0", t)),
        (e = T[r.p] - x[r.p] - Y + Ke + i - N),
        l && Kr(l, N, r, t - N < 20 || (l._isStart && N > 20)),
        (t -= t - N));
    }
    if ((v && ((p[v] = e || -0.001), e < 0 && (e = 0)), u)) {
      var ke = e + t,
        Ie = u._isStart;
      ((X = "scroll" + r.d2),
        Kr(
          u,
          ke,
          r,
          (Ie && ke > 20) ||
            (!Ie && (M ? Math.max(L[X], qe[X]) : u.parentNode[X]) <= ke + 1),
        ),
        M &&
          ((x = wt(l)),
          M && (u.style[r.op.p] = x[r.op.p] - r.op.m - u._offset + ae)));
    }
    return (
      c &&
        W &&
        ((X = wt(W)),
        c.seek(h),
        (ie = wt(W)),
        (c._caScrollDist = X[r.p] - ie[r.p]),
        (e = (e / c._caScrollDist) * h)),
      c && c.seek(U),
      c ? e : Math.round(e)
    );
  },
  Ai = /(webkit|moz|length|cssText|inset)/i,
  Jn = function (e, n, t, r) {
    if (e.parentNode !== n) {
      var i = e.style,
        u,
        l;
      if (n === L) {
        ((e._stOrig = i.cssText), (l = Qe(e)));
        for (u in l)
          !+u &&
            !Ai.test(u) &&
            l[u] &&
            typeof i[u] == "string" &&
            u !== "0" &&
            (i[u] = l[u]);
        ((i.top = t), (i.left = r));
      } else i.cssText = e._stOrig;
      ((d.core.getCache(e).uncache = 1), n.appendChild(e));
    }
  },
  vi = function (e, n, t) {
    var r = n,
      i = r;
    return function (u) {
      var l = Math.round(e());
      return (
        l !== r &&
          l !== i &&
          Math.abs(l - r) > 3 &&
          Math.abs(l - i) > 3 &&
          ((u = l), t && t()),
        (i = r),
        (r = Math.round(u)),
        r
      );
    };
  },
  Ur = function (e, n, t) {
    var r = {};
    ((r[n.p] = "+=" + t), d.set(e, r));
  },
  Qn = function (e, n) {
    var t = yt(e, n),
      r = "_scroll" + n.p2,
      i = function u(l, p, x, Y, M) {
        var h = u.tween,
          c = p.onComplete,
          v = {};
        x = x || t();
        var U = vi(t, x, function () {
          (h.kill(), (u.tween = 0));
        });
        return (
          (M = (Y && M) || 0),
          (Y = Y || l - x),
          h && h.kill(),
          (p[r] = l),
          (p.inherit = !1),
          (p.modifiers = v),
          (v[r] = function () {
            return U(x + Y * h.ratio + M * h.ratio * h.ratio);
          }),
          (p.onUpdate = function () {
            (k.cache++, u.tween && Ct());
          }),
          (p.onComplete = function () {
            ((u.tween = 0), c && c.call(h));
          }),
          (h = u.tween = d.to(e, p)),
          h
        );
      };
    return (
      (e[r] = t),
      (t.wheelHandler = function () {
        return i.tween && i.tween.kill() && (i.tween = 0);
      }),
      de(e, "wheel", t.wheelHandler),
      R.isTouch && de(e, "touchmove", t.wheelHandler),
      i
    );
  },
  R = (function () {
    function o(n, t) {
      (tr ||
        o.register(d) ||
        console.warn("Please gsap.registerPlugin(ScrollTrigger)"),
        _n(this),
        this.init(n, t));
    }
    var e = o.prototype;
    return (
      (e.init = function (t, r) {
        if (
          ((this.progress = this.start = 0),
          this.vars && this.kill(!0, !0),
          !vr)
        ) {
          this.update = this.refresh = this.kill = ct;
          return;
        }
        t = Un(Ve(t) || xr(t) || t.nodeType ? { trigger: t } : t, Hr);
        var i = t,
          u = i.onUpdate,
          l = i.toggleClass,
          p = i.id,
          x = i.onToggle,
          Y = i.onRefresh,
          M = i.scrub,
          h = i.trigger,
          c = i.pin,
          v = i.pinSpacing,
          U = i.invalidateOnRefresh,
          X = i.anticipatePin,
          ie = i.onScrubComplete,
          W = i.onSnapComplete,
          ge = i.once,
          T = i.snap,
          Ke = i.pinReparent,
          N = i.pinSpacer,
          b = i.containerAnimation,
          ke = i.fastScrollEnd,
          Ie = i.preventOverlaps,
          g =
            t.horizontal || (t.containerAnimation && t.horizontal !== !1)
              ? we
              : re,
          ue = !M && M !== 0,
          y = Ye(t.scroller || E),
          pt = d.core.getCache(y),
          oe = Ut(y),
          Pe =
            ("pinType" in t
              ? t.pinType
              : bt(y, "pinType") || (oe && "fixed")) === "fixed",
          Ee = [t.onEnter, t.onLeave, t.onEnterBack, t.onLeaveBack],
          z = ue && t.toggleActions.split(" "),
          Q = "markers" in t ? t.markers : Hr.markers,
          j = oe ? 0 : parseFloat(Qe(y)["border" + g.p2 + or]) || 0,
          s = this,
          se =
            t.onRefreshInit &&
            function () {
              return t.onRefreshInit(s);
            },
          At = Ci(y, oe, g),
          St = Si(y, oe),
          Ze = 0,
          dt = 0,
          Me = 0,
          ee = yt(y, g),
          De,
          me,
          Tt,
          Re,
          Xe,
          A,
          K,
          Be,
          Ne,
          a,
          He,
          gt,
          kt,
          Z,
          ht,
          Pt,
          Yt,
          ce,
          Et,
          $,
          et,
          $e,
          _t,
          sr,
          te,
          Mr,
          vt,
          Kt,
          Zt,
          Mt,
          Ft,
          D,
          Lt,
          tt,
          rt,
          nt,
          zt,
          $t,
          mt;
        if (
          ((s._startClamp = s._endClamp = !1),
          (s._dir = g),
          (X *= 45),
          (s.scroller = y),
          (s.scroll = b ? b.time.bind(b) : ee),
          (Re = ee()),
          (s.vars = t),
          (r = r || t.animation),
          "refreshPriority" in t &&
            ((ri = 1), t.refreshPriority === -9999 && (kr = s)),
          (pt.tweenScroll = pt.tweenScroll || {
            top: Qn(y, re),
            left: Qn(y, we),
          }),
          (s.tweenTo = De = pt.tweenScroll[g.p]),
          (s.scrubDuration = function (f) {
            ((Lt = xr(f) && f),
              Lt
                ? D
                  ? D.duration(f)
                  : (D = d.to(r, {
                      ease: "expo",
                      totalProgress: "+=0",
                      inherit: !1,
                      duration: Lt,
                      paused: !0,
                      onComplete: function () {
                        return ie && ie(s);
                      },
                    }))
                : (D && D.progress(1).kill(), (D = 0)));
          }),
          r &&
            ((r.vars.lazy = !1),
            (r._initted && !s.isReverted) ||
              (r.vars.immediateRender !== !1 &&
                t.immediateRender !== !1 &&
                r.duration() &&
                r.render(0, !0, !0)),
            (s.animation = r.pause()),
            (r.scrollTrigger = s),
            s.scrubDuration(M),
            (Mt = 0),
            p || (p = r.vars.id)),
          T &&
            ((!Bt(T) || T.push) && (T = { snapTo: T }),
            "scrollBehavior" in L.style &&
              d.set(oe ? [L, qe] : y, { scrollBehavior: "auto" }),
            k.forEach(function (f) {
              return (
                Te(f) &&
                f.target === (oe ? B.scrollingElement || qe : y) &&
                (f.smooth = !1)
              );
            }),
            (Tt = Te(T.snapTo)
              ? T.snapTo
              : T.snapTo === "labels"
                ? ki(r)
                : T.snapTo === "labelsDirectional"
                  ? Pi(r)
                  : T.directional !== !1
                    ? function (f, w) {
                        return Pn(T.snapTo)(
                          f,
                          Se() - dt < 500 ? 0 : w.direction,
                        );
                      }
                    : d.utils.snap(T.snapTo)),
            (tt = T.duration || { min: 0.1, max: 2 }),
            (tt = Bt(tt) ? br(tt.min, tt.max) : br(tt, tt)),
            (rt = d
              .delayedCall(T.delay || Lt / 2 || 0.1, function () {
                var f = ee(),
                  w = Se() - dt < 500,
                  _ = De.tween;
                if (
                  (w || Math.abs(s.getVelocity()) < 10) &&
                  !_ &&
                  !rn &&
                  Ze !== f
                ) {
                  var C = (f - A) / Z,
                    fe = r && !ue ? r.totalProgress() : C,
                    O = w ? 0 : ((fe - Ft) / (Se() - _r)) * 1e3 || 0,
                    J = d.utils.clamp(-C, 1 - C, (jt(O / 2) * O) / 0.185),
                    xe = C + (T.inertia === !1 ? 0 : J),
                    V,
                    H,
                    I = T,
                    it = I.onStart,
                    G = I.onInterrupt,
                    We = I.onComplete;
                  if (
                    ((V = Tt(xe, s)),
                    xr(V) || (V = xe),
                    (H = Math.max(0, Math.round(A + V * Z))),
                    f <= K && f >= A && H !== f)
                  ) {
                    if (_ && !_._initted && _.data <= jt(H - f)) return;
                    (T.inertia === !1 && (J = V - C),
                      De(
                        H,
                        {
                          duration: tt(
                            jt(
                              (Math.max(jt(xe - fe), jt(V - fe)) * 0.185) /
                                O /
                                0.05 || 0,
                            ),
                          ),
                          ease: T.ease || "power3",
                          data: jt(H - f),
                          onInterrupt: function () {
                            return rt.restart(!0) && G && G(s);
                          },
                          onComplete: function () {
                            (s.update(),
                              (Ze = ee()),
                              r &&
                                !ue &&
                                (D
                                  ? D.resetTo(
                                      "totalProgress",
                                      V,
                                      r._tTime / r._tDur,
                                    )
                                  : r.progress(V)),
                              (Mt = Ft =
                                r && !ue ? r.totalProgress() : s.progress),
                              W && W(s),
                              We && We(s));
                          },
                        },
                        f,
                        J * Z,
                        H - f - J * Z,
                      ),
                      it && it(s, De.tween));
                  }
                } else s.isActive && Ze !== f && rt.restart(!0);
              })
              .pause())),
          p && (mn[p] = s),
          (h = s.trigger = Ye(h || (c !== !0 && c))),
          (mt = h && h._gsap && h._gsap.stRevert),
          mt && (mt = mt(s)),
          (c = c === !0 ? h : Ye(c)),
          Ve(l) && (l = { targets: h, className: l }),
          c &&
            (v === !1 ||
              v === Je ||
              (v =
                !v &&
                c.parentNode &&
                c.parentNode.style &&
                Qe(c.parentNode).display === "flex"
                  ? !1
                  : ne),
            (s.pin = c),
            (me = d.core.getCache(c)),
            me.spacer
              ? (ht = me.pinState)
              : (N &&
                  ((N = Ye(N)),
                  N && !N.nodeType && (N = N.current || N.nativeElement),
                  (me.spacerIsNative = !!N),
                  N && (me.spacerState = Gr(N))),
                (me.spacer = ce = N || B.createElement("div")),
                ce.classList.add("pin-spacer"),
                p && ce.classList.add("pin-spacer-" + p),
                (me.pinState = ht = Gr(c))),
            t.force3D !== !1 && d.set(c, { force3D: !0 }),
            (s.spacer = ce = me.spacer),
            (Zt = Qe(c)),
            (sr = Zt[v + g.os2]),
            ($ = d.getProperty(c)),
            (et = d.quickSetter(c, g.a, ae)),
            pn(c, ce, Zt),
            (Yt = Gr(c))),
          Q)
        ) {
          ((gt = Bt(Q) ? Un(Q, Vn) : Vn),
            (a = Wr("scroller-start", p, y, g, gt, 0)),
            (He = Wr("scroller-end", p, y, g, gt, 0, a)),
            (Et = a["offset" + g.op.d2]));
          var lr = Ye(bt(y, "content") || y);
          ((Be = this.markerStart = Wr("start", p, lr, g, gt, Et, 0, b)),
            (Ne = this.markerEnd = Wr("end", p, lr, g, gt, Et, 0, b)),
            b && ($t = d.quickSetter([Be, Ne], g.a, ae)),
            !Pe &&
              !(st.length && bt(y, "fixedMarkers") === !0) &&
              (Ti(oe ? L : y),
              d.set([a, He], { force3D: !0 }),
              (Mr = d.quickSetter(a, g.a, ae)),
              (Kt = d.quickSetter(He, g.a, ae))));
        }
        if (b) {
          var S = b.vars.onUpdate,
            m = b.vars.onUpdateParams;
          b.eventCallback("onUpdate", function () {
            (s.update(0, 0, 1), S && S.apply(b, m || []));
          });
        }
        if (
          ((s.previous = function () {
            return P[P.indexOf(s) - 1];
          }),
          (s.next = function () {
            return P[P.indexOf(s) + 1];
          }),
          (s.revert = function (f, w) {
            if (!w) return s.kill(!0);
            var _ = f !== !1 || !s.enabled,
              C = Ce;
            _ !== s.isReverted &&
              (_ &&
                ((nt = Math.max(ee(), s.scroll.rec || 0)),
                (Me = s.progress),
                (zt = r && r.progress())),
              Be &&
                [Be, Ne, a, He].forEach(function (fe) {
                  return (fe.style.display = _ ? "none" : "block");
                }),
              _ && ((Ce = s), s.update(_)),
              c &&
                (!Ke || !s.isActive) &&
                (_ ? Di(c, ce, ht) : pn(c, ce, Qe(c), te)),
              _ || s.update(_),
              (Ce = C),
              (s.isReverted = _));
          }),
          (s.refresh = function (f, w, _, C) {
            if (!((Ce || !s.enabled) && !w)) {
              if (c && f && je) {
                de(o, "scrollEnd", di);
                return;
              }
              (!ze && se && se(s),
                (Ce = s),
                De.tween && !_ && (De.tween.kill(), (De.tween = 0)),
                D && D.pause(),
                U && r && r.revert({ kill: !1 }).invalidate(),
                s.isReverted || s.revert(!0, !0),
                (s._subPinOffset = !1));
              var fe = At(),
                O = St(),
                J = b ? b.duration() : ft(y, g),
                xe = Z <= 0.01,
                V = 0,
                H = C || 0,
                I = Bt(_) ? _.end : t.end,
                it = t.endTrigger || h,
                G = Bt(_)
                  ? _.start
                  : t.start || (t.start === 0 || !h ? 0 : c ? "0 0" : "0 100%"),
                We = (s.pinnedContainer =
                  t.pinnedContainer && Ye(t.pinnedContainer, s)),
                lt = (h && Math.max(0, P.indexOf(s))) || 0,
                he = lt,
                _e,
                ye,
                It,
                Dr,
                be,
                le,
                at,
                nn,
                Mn,
                ar,
                ut,
                ur,
                Rr;
              for (
                Q &&
                Bt(_) &&
                ((ur = d.getProperty(a, g.p)), (Rr = d.getProperty(He, g.p)));
                he-- > 0;
              )
                ((le = P[he]),
                  le.end || le.refresh(0, 1) || (Ce = s),
                  (at = le.pin),
                  at &&
                    (at === h || at === c || at === We) &&
                    !le.isReverted &&
                    (ar || (ar = []), ar.unshift(le), le.revert(!0, !0)),
                  le !== P[he] && (lt--, he--));
              for (
                Te(G) && (G = G(s)),
                  G = Nn(G, "start", s),
                  A =
                    $n(
                      G,
                      h,
                      fe,
                      g,
                      ee(),
                      Be,
                      a,
                      s,
                      O,
                      j,
                      Pe,
                      J,
                      b,
                      s._startClamp && "_startClamp",
                    ) || (c ? -0.001 : 0),
                  Te(I) && (I = I(s)),
                  Ve(I) &&
                    !I.indexOf("+=") &&
                    (~I.indexOf(" ")
                      ? (I = (Ve(G) ? G.split(" ")[0] : "") + I)
                      : ((V = qr(I.substr(2), fe)),
                        (I = Ve(G)
                          ? G
                          : (b
                              ? d.utils.mapRange(
                                  0,
                                  b.duration(),
                                  b.scrollTrigger.start,
                                  b.scrollTrigger.end,
                                  A,
                                )
                              : A) + V),
                        (it = h))),
                  I = Nn(I, "end", s),
                  K =
                    Math.max(
                      A,
                      $n(
                        I || (it ? "100% 0" : J),
                        it,
                        fe,
                        g,
                        ee() + V,
                        Ne,
                        He,
                        s,
                        O,
                        j,
                        Pe,
                        J,
                        b,
                        s._endClamp && "_endClamp",
                      ),
                    ) || -0.001,
                  V = 0,
                  he = lt;
                he--;
              )
                ((le = P[he]),
                  (at = le.pin),
                  at &&
                    le.start - le._pinPush <= A &&
                    !b &&
                    le.end > 0 &&
                    ((_e =
                      le.end -
                      (s._startClamp ? Math.max(0, le.start) : le.start)),
                    ((at === h && le.start - le._pinPush < A) || at === We) &&
                      isNaN(G) &&
                      (V += _e * (1 - le.progress)),
                    at === c && (H += _e)));
              if (
                ((A += V),
                (K += V),
                s._startClamp && (s._startClamp += V),
                s._endClamp &&
                  !ze &&
                  ((s._endClamp = K || -0.001), (K = Math.min(K, ft(y, g)))),
                (Z = K - A || ((A -= 0.01) && 0.001)),
                xe && (Me = d.utils.clamp(0, 1, d.utils.normalize(A, K, nt))),
                (s._pinPush = H),
                Be &&
                  V &&
                  ((_e = {}),
                  (_e[g.a] = "+=" + V),
                  We && (_e[g.p] = "-=" + ee()),
                  d.set([Be, Ne], _e)),
                c && !(vn && s.end >= ft(y, g)))
              )
                ((_e = Qe(c)),
                  (Dr = g === re),
                  (It = ee()),
                  ($e = parseFloat($(g.a)) + H),
                  !J &&
                    K > 1 &&
                    ((ut = (oe ? B.scrollingElement || qe : y).style),
                    (ut = {
                      style: ut,
                      value: ut["overflow" + g.a.toUpperCase()],
                    }),
                    oe &&
                      Qe(L)["overflow" + g.a.toUpperCase()] !== "scroll" &&
                      (ut.style["overflow" + g.a.toUpperCase()] = "scroll")),
                  pn(c, ce, _e),
                  (Yt = Gr(c)),
                  (ye = wt(c, !0)),
                  (nn = Pe && yt(y, Dr ? we : re)()),
                  v
                    ? ((te = [v + g.os2, Z + H + ae]),
                      (te.t = ce),
                      (he = v === ne ? en(c, g) + Z + H : 0),
                      he &&
                        (te.push(g.d, he + ae),
                        ce.style.flexBasis !== "auto" &&
                          (ce.style.flexBasis = he + ae)),
                      ir(te),
                      We &&
                        P.forEach(function (cr) {
                          cr.pin === We &&
                            cr.vars.pinSpacing !== !1 &&
                            (cr._subPinOffset = !0);
                        }),
                      Pe && ee(nt))
                    : ((he = en(c, g)),
                      he &&
                        ce.style.flexBasis !== "auto" &&
                        (ce.style.flexBasis = he + ae)),
                  Pe &&
                    ((be = {
                      top: ye.top + (Dr ? It - A : nn) + ae,
                      left: ye.left + (Dr ? nn : It - A) + ae,
                      boxSizing: "border-box",
                      position: "fixed",
                    }),
                    (be[Ht] = be["max" + or] = Math.ceil(ye.width) + ae),
                    (be[Wt] = be["max" + kn] = Math.ceil(ye.height) + ae),
                    (be[Je] =
                      be[Je + Sr] =
                      be[Je + wr] =
                      be[Je + Tr] =
                      be[Je + Cr] =
                        "0"),
                    (be[ne] = _e[ne]),
                    (be[ne + Sr] = _e[ne + Sr]),
                    (be[ne + wr] = _e[ne + wr]),
                    (be[ne + Tr] = _e[ne + Tr]),
                    (be[ne + Cr] = _e[ne + Cr]),
                    (Pt = Oi(ht, be, Ke)),
                    ze && ee(0)),
                  r
                    ? ((Mn = r._initted),
                      ln(1),
                      r.render(r.duration(), !0, !0),
                      (_t = $(g.a) - $e + Z + H),
                      (vt = Math.abs(Z - _t) > 1),
                      Pe && vt && Pt.splice(Pt.length - 2, 2),
                      r.render(0, !0, !0),
                      Mn || r.invalidate(!0),
                      r.parent || r.totalTime(r.totalTime()),
                      ln(0))
                    : (_t = Z),
                  ut &&
                    (ut.value
                      ? (ut.style["overflow" + g.a.toUpperCase()] = ut.value)
                      : ut.style.removeProperty("overflow-" + g.a)));
              else if (h && ee() && !b)
                for (ye = h.parentNode; ye && ye !== L; )
                  (ye._pinOffset &&
                    ((A -= ye._pinOffset), (K -= ye._pinOffset)),
                    (ye = ye.parentNode));
              (ar &&
                ar.forEach(function (cr) {
                  return cr.revert(!1, !0);
                }),
                (s.start = A),
                (s.end = K),
                (Re = Xe = ze ? nt : ee()),
                !b && !ze && (Re < nt && ee(nt), (s.scroll.rec = 0)),
                s.revert(!1, !0),
                (dt = Se()),
                rt && ((Ze = -1), rt.restart(!0)),
                (Ce = 0),
                r &&
                  ue &&
                  (r._initted || zt) &&
                  r.progress() !== zt &&
                  r.progress(zt || 0, !0).render(r.time(), !0, !0),
                (xe || Me !== s.progress || b || U || (r && !r._initted)) &&
                  (r &&
                    !ue &&
                    r.totalProgress(
                      b && A < -0.001 && !Me ? d.utils.normalize(A, K, 0) : Me,
                      !0,
                    ),
                  (s.progress = xe || (Re - A) / Z === Me ? 0 : Me)),
                c && v && (ce._pinOffset = Math.round(s.progress * _t)),
                D && D.invalidate(),
                isNaN(ur) ||
                  ((ur -= d.getProperty(a, g.p)),
                  (Rr -= d.getProperty(He, g.p)),
                  Ur(a, g, ur),
                  Ur(Be, g, ur - (C || 0)),
                  Ur(He, g, Rr),
                  Ur(Ne, g, Rr - (C || 0))),
                xe && !ze && s.update(),
                Y && !ze && !kt && ((kt = !0), Y(s), (kt = !1)));
            }
          }),
          (s.getVelocity = function () {
            return ((ee() - Xe) / (Se() - _r)) * 1e3 || 0;
          }),
          (s.endAnimation = function () {
            (hr(s.callbackAnimation),
              r &&
                (D
                  ? D.progress(1)
                  : r.paused()
                    ? ue || hr(r, s.direction < 0, 1)
                    : hr(r, r.reversed())));
          }),
          (s.labelToScroll = function (f) {
            return (
              (r &&
                r.labels &&
                (A || s.refresh() || A) + (r.labels[f] / r.duration()) * Z) ||
              0
            );
          }),
          (s.getTrailing = function (f) {
            var w = P.indexOf(s),
              _ = s.direction > 0 ? P.slice(0, w).reverse() : P.slice(w + 1);
            return (
              Ve(f)
                ? _.filter(function (C) {
                    return C.vars.preventOverlaps === f;
                  })
                : _
            ).filter(function (C) {
              return s.direction > 0 ? C.end <= A : C.start >= K;
            });
          }),
          (s.update = function (f, w, _) {
            if (!(b && !_ && !f)) {
              var C = ze === !0 ? nt : s.scroll(),
                fe = f ? 0 : (C - A) / Z,
                O = fe < 0 ? 0 : fe > 1 ? 1 : fe || 0,
                J = s.progress,
                xe,
                V,
                H,
                I,
                it,
                G,
                We,
                lt;
              if (
                (w &&
                  ((Xe = Re),
                  (Re = b ? ee() : C),
                  T && ((Ft = Mt), (Mt = r && !ue ? r.totalProgress() : O))),
                X &&
                  c &&
                  !Ce &&
                  !Ir &&
                  je &&
                  (!O && A < C + ((C - Xe) / (Se() - _r)) * X
                    ? (O = 1e-4)
                    : O === 1 &&
                      K > C + ((C - Xe) / (Se() - _r)) * X &&
                      (O = 0.9999)),
                O !== J && s.enabled)
              ) {
                if (
                  ((xe = s.isActive = !!O && O < 1),
                  (V = !!J && J < 1),
                  (G = xe !== V),
                  (it = G || !!O != !!J),
                  (s.direction = O > J ? 1 : -1),
                  (s.progress = O),
                  it &&
                    !Ce &&
                    ((H = O && !J ? 0 : O === 1 ? 1 : J === 1 ? 2 : 3),
                    ue &&
                      ((I = (!G && z[H + 1] !== "none" && z[H + 1]) || z[H]),
                      (lt =
                        r && (I === "complete" || I === "reset" || I in r)))),
                  Ie &&
                    (G || lt) &&
                    (lt || M || !r) &&
                    (Te(Ie)
                      ? Ie(s)
                      : s.getTrailing(Ie).forEach(function (It) {
                          return It.endAnimation();
                        })),
                  ue ||
                    (D && !Ce && !Ir
                      ? (D._dp._time - D._start !== D._time &&
                          D.render(D._dp._time - D._start),
                        D.resetTo
                          ? D.resetTo("totalProgress", O, r._tTime / r._tDur)
                          : ((D.vars.totalProgress = O),
                            D.invalidate().restart()))
                      : r && r.totalProgress(O, !!(Ce && (dt || f)))),
                  c)
                ) {
                  if ((f && v && (ce.style[v + g.os2] = sr), !Pe))
                    et(mr($e + _t * O));
                  else if (it) {
                    if (
                      ((We = !f && O > J && K + 1 > C && C + 1 >= ft(y, g)), Ke)
                    )
                      if (!f && (xe || We)) {
                        var he = wt(c, !0),
                          _e = C - A;
                        Jn(
                          c,
                          L,
                          he.top + (g === re ? _e : 0) + ae,
                          he.left + (g === re ? 0 : _e) + ae,
                        );
                      } else Jn(c, ce);
                    (ir(xe || We ? Pt : Yt),
                      (vt && O < 1 && xe) ||
                        et($e + (O === 1 && !We ? _t : 0)));
                  }
                }
                (T && !De.tween && !Ce && !Ir && rt.restart(!0),
                  l &&
                    (G || (ge && O && (O < 1 || !an))) &&
                    Pr(l.targets).forEach(function (It) {
                      return It.classList[xe || ge ? "add" : "remove"](
                        l.className,
                      );
                    }),
                  u && !ue && !f && u(s),
                  it && !Ce
                    ? (ue &&
                        (lt &&
                          (I === "complete"
                            ? r.pause().totalProgress(1)
                            : I === "reset"
                              ? r.restart(!0).pause()
                              : I === "restart"
                                ? r.restart(!0)
                                : r[I]()),
                        u && u(s)),
                      (G || !an) &&
                        (x && G && cn(s, x),
                        Ee[H] && cn(s, Ee[H]),
                        ge && (O === 1 ? s.kill(!1, 1) : (Ee[H] = 0)),
                        G || ((H = O === 1 ? 1 : 3), Ee[H] && cn(s, Ee[H]))),
                      ke &&
                        !xe &&
                        Math.abs(s.getVelocity()) > (xr(ke) ? ke : 2500) &&
                        (hr(s.callbackAnimation),
                        D ? D.progress(1) : hr(r, I === "reverse" ? 1 : !O, 1)))
                    : ue && u && !Ce && u(s));
              }
              if (Kt) {
                var ye = b ? (C / b.duration()) * (b._caScrollDist || 0) : C;
                (Mr(ye + (a._isFlipped ? 1 : 0)), Kt(ye));
              }
              $t && $t((-C / b.duration()) * (b._caScrollDist || 0));
            }
          }),
          (s.enable = function (f, w) {
            s.enabled ||
              ((s.enabled = !0),
              de(y, "resize", yr),
              oe || de(y, "scroll", er),
              se && de(o, "refreshInit", se),
              f !== !1 && ((s.progress = Me = 0), (Re = Xe = Ze = ee())),
              w !== !1 && s.refresh());
          }),
          (s.getTween = function (f) {
            return f && De ? De.tween : D;
          }),
          (s.setPositions = function (f, w, _, C) {
            if (b) {
              var fe = b.scrollTrigger,
                O = b.duration(),
                J = fe.end - fe.start;
              ((f = fe.start + (J * f) / O), (w = fe.start + (J * w) / O));
            }
            (s.refresh(
              !1,
              !1,
              {
                start: Hn(f, _ && !!s._startClamp),
                end: Hn(w, _ && !!s._endClamp),
              },
              C,
            ),
              s.update());
          }),
          (s.adjustPinSpacing = function (f) {
            if (te && f) {
              var w = te.indexOf(g.d) + 1;
              ((te[w] = parseFloat(te[w]) + f + ae),
                (te[1] = parseFloat(te[1]) + f + ae),
                ir(te));
            }
          }),
          (s.disable = function (f, w) {
            if (
              s.enabled &&
              (f !== !1 && s.revert(!0, !0),
              (s.enabled = s.isActive = !1),
              w || (D && D.pause()),
              (nt = 0),
              me && (me.uncache = 1),
              se && pe(o, "refreshInit", se),
              rt && (rt.pause(), De.tween && De.tween.kill() && (De.tween = 0)),
              !oe)
            ) {
              for (var _ = P.length; _--; )
                if (P[_].scroller === y && P[_] !== s) return;
              (pe(y, "resize", yr), oe || pe(y, "scroll", er));
            }
          }),
          (s.kill = function (f, w) {
            (s.disable(f, w), D && !w && D.kill(), p && delete mn[p]);
            var _ = P.indexOf(s);
            (_ >= 0 && P.splice(_, 1),
              _ === Le && Zr > 0 && Le--,
              (_ = 0),
              P.forEach(function (C) {
                return C.scroller === s.scroller && (_ = 1);
              }),
              _ || ze || (s.scroll.rec = 0),
              r &&
                ((r.scrollTrigger = null),
                f && r.revert({ kill: !1 }),
                w || r.kill()),
              Be &&
                [Be, Ne, a, He].forEach(function (C) {
                  return C.parentNode && C.parentNode.removeChild(C);
                }),
              kr === s && (kr = 0),
              c &&
                (me && (me.uncache = 1),
                (_ = 0),
                P.forEach(function (C) {
                  return C.pin === c && _++;
                }),
                _ || (me.spacer = 0)),
              t.onKill && t.onKill(s));
          }),
          P.push(s),
          s.enable(!1, !1),
          mt && mt(s),
          r && r.add && !Z)
        ) {
          var F = s.update;
          ((s.update = function () {
            ((s.update = F), k.cache++, A || K || s.refresh());
          }),
            d.delayedCall(0.01, s.update),
            (Z = 0.01),
            (A = K = 0));
        } else s.refresh();
        c && Mi();
      }),
      (o.register = function (t) {
        return (
          tr ||
            ((d = t || li()), si() && window.document && o.enable(), (tr = vr)),
          tr
        );
      }),
      (o.defaults = function (t) {
        if (t) for (var r in t) Hr[r] = t[r];
        return Hr;
      }),
      (o.disable = function (t, r) {
        ((vr = 0),
          P.forEach(function (u) {
            return u[r ? "kill" : "disable"](t);
          }),
          pe(E, "wheel", er),
          pe(B, "scroll", er),
          clearInterval(zr),
          pe(B, "touchcancel", ct),
          pe(L, "touchstart", ct),
          Br(pe, B, "pointerdown,touchstart,mousedown", Wn),
          Br(pe, B, "pointerup,touchend,mouseup", Gn),
          Qr.kill(),
          Xr(pe));
        for (var i = 0; i < k.length; i += 3)
          (Nr(pe, k[i], k[i + 1]), Nr(pe, k[i], k[i + 2]));
      }),
      (o.enable = function () {
        if (
          ((E = window),
          (B = document),
          (qe = B.documentElement),
          (L = B.body),
          d &&
            ((Pr = d.utils.toArray),
            (br = d.utils.clamp),
            (_n = d.core.context || ct),
            (ln = d.core.suppressOverwrites || ct),
            (wn = E.history.scrollRestoration || "auto"),
            (xn = E.pageYOffset || 0),
            d.core.globals("ScrollTrigger", o),
            L))
        ) {
          ((vr = 1),
            (nr = document.createElement("div")),
            (nr.style.height = "100vh"),
            (nr.style.position = "absolute"),
            _i(),
            wi(),
            q.register(d),
            (o.isTouch = q.isTouch),
            (Ot =
              q.isTouch && /(iPad|iPhone|iPod|Mac)/g.test(navigator.userAgent)),
            (hn = q.isTouch === 1),
            de(E, "wheel", er),
            (bn = [E, B, qe, L]),
            d.matchMedia
              ? ((o.matchMedia = function (x) {
                  var Y = d.matchMedia(),
                    M;
                  for (M in x) Y.add(M, x[M]);
                  return Y;
                }),
                d.addEventListener("matchMediaInit", function () {
                  return En();
                }),
                d.addEventListener("matchMediaRevert", function () {
                  return gi();
                }),
                d.addEventListener("matchMedia", function () {
                  (Nt(0, 1), qt("matchMedia"));
                }),
                d.matchMedia().add("(orientation: portrait)", function () {
                  return (fn(), fn);
                }))
              : console.warn("Requires GSAP 3.11.0 or later"),
            fn(),
            de(B, "scroll", er));
          var t = L.hasAttribute("style"),
            r = L.style,
            i = r.borderTopStyle,
            u = d.core.Animation.prototype,
            l,
            p;
          for (
            u.revert ||
              Object.defineProperty(u, "revert", {
                value: function () {
                  return this.time(-0.01, !0);
                },
              }),
              r.borderTopStyle = "solid",
              l = wt(L),
              re.m = Math.round(l.top + re.sc()) || 0,
              we.m = Math.round(l.left + we.sc()) || 0,
              i ? (r.borderTopStyle = i) : r.removeProperty("border-top-style"),
              t || (L.setAttribute("style", ""), L.removeAttribute("style")),
              zr = setInterval(qn, 250),
              d.delayedCall(0.5, function () {
                return (Ir = 0);
              }),
              de(B, "touchcancel", ct),
              de(L, "touchstart", ct),
              Br(de, B, "pointerdown,touchstart,mousedown", Wn),
              Br(de, B, "pointerup,touchend,mouseup", Gn),
              gn = d.utils.checkPrefix("transform"),
              $r.push(gn),
              tr = Se(),
              Qr = d.delayedCall(0.2, Nt).pause(),
              rr = [
                B,
                "visibilitychange",
                function () {
                  var x = E.innerWidth,
                    Y = E.innerHeight;
                  B.hidden
                    ? ((Xn = x), (Bn = Y))
                    : (Xn !== x || Bn !== Y) && yr();
                },
                B,
                "DOMContentLoaded",
                Nt,
                E,
                "load",
                Nt,
                E,
                "resize",
                yr,
              ],
              Xr(de),
              P.forEach(function (x) {
                return x.enable(0, 1);
              }),
              p = 0;
            p < k.length;
            p += 3
          )
            (Nr(pe, k[p], k[p + 1]), Nr(pe, k[p], k[p + 2]));
        }
      }),
      (o.config = function (t) {
        "limitCallbacks" in t && (an = !!t.limitCallbacks);
        var r = t.syncInterval;
        ((r && clearInterval(zr)) || ((zr = r) && setInterval(qn, r)),
          "ignoreMobileResize" in t &&
            (hn = o.isTouch === 1 && t.ignoreMobileResize),
          "autoRefreshEvents" in t &&
            (Xr(pe) || Xr(de, t.autoRefreshEvents || "none"),
            (ni = (t.autoRefreshEvents + "").indexOf("resize") === -1)));
      }),
      (o.scrollerProxy = function (t, r) {
        var i = Ye(t),
          u = k.indexOf(i),
          l = Ut(i);
        (~u && k.splice(u, l ? 6 : 2),
          r && (l ? st.unshift(E, r, L, r, qe, r) : st.unshift(i, r)));
      }),
      (o.clearMatchMedia = function (t) {
        P.forEach(function (r) {
          return r._ctx && r._ctx.query === t && r._ctx.kill(!0, !0);
        });
      }),
      (o.isInViewport = function (t, r, i) {
        var u = (Ve(t) ? Ye(t) : t).getBoundingClientRect(),
          l = u[i ? Ht : Wt] * r || 0;
        return i
          ? u.right - l > 0 && u.left + l < E.innerWidth
          : u.bottom - l > 0 && u.top + l < E.innerHeight;
      }),
      (o.positionInViewport = function (t, r, i) {
        Ve(t) && (t = Ye(t));
        var u = t.getBoundingClientRect(),
          l = u[i ? Ht : Wt],
          p =
            r == null
              ? l / 2
              : r in tn
                ? tn[r] * l
                : ~r.indexOf("%")
                  ? (parseFloat(r) * l) / 100
                  : parseFloat(r) || 0;
        return i ? (u.left + p) / E.innerWidth : (u.top + p) / E.innerHeight;
      }),
      (o.killAll = function (t) {
        if (
          (P.slice(0).forEach(function (i) {
            return i.vars.id !== "ScrollSmoother" && i.kill();
          }),
          t !== !0)
        ) {
          var r = Vt.killAll || [];
          ((Vt = {}),
            r.forEach(function (i) {
              return i();
            }));
        }
      }),
      o
    );
  })();
R.version = "3.12.7";
R.saveStyles = function (o) {
  return o
    ? Pr(o).forEach(function (e) {
        if (e && e.style) {
          var n = Ue.indexOf(e);
          (n >= 0 && Ue.splice(n, 5),
            Ue.push(
              e,
              e.style.cssText,
              e.getBBox && e.getAttribute("transform"),
              d.core.getCache(e),
              _n(),
            ));
        }
      })
    : Ue;
};
R.revert = function (o, e) {
  return En(!o, e);
};
R.create = function (o, e) {
  return new R(o, e);
};
R.refresh = function (o) {
  return o ? yr(!0) : (tr || R.register()) && Nt(!0);
};
R.update = function (o) {
  return ++k.cache && Ct(o === !0 ? 2 : 0);
};
R.clearScrollMemory = hi;
R.maxScroll = function (o, e) {
  return ft(o, e ? we : re);
};
R.getScrollFunc = function (o, e) {
  return yt(Ye(o), e ? we : re);
};
R.getById = function (o) {
  return mn[o];
};
R.getAll = function () {
  return P.filter(function (o) {
    return o.vars.id !== "ScrollSmoother";
  });
};
R.isScrolling = function () {
  return !!je;
};
R.snapDirectional = Pn;
R.addEventListener = function (o, e) {
  var n = Vt[o] || (Vt[o] = []);
  ~n.indexOf(e) || n.push(e);
};
R.removeEventListener = function (o, e) {
  var n = Vt[o],
    t = n && n.indexOf(e);
  t >= 0 && n.splice(t, 1);
};
R.batch = function (o, e) {
  var n = [],
    t = {},
    r = e.interval || 0.016,
    i = e.batchMax || 1e9,
    u = function (x, Y) {
      var M = [],
        h = [],
        c = d
          .delayedCall(r, function () {
            (Y(M, h), (M = []), (h = []));
          })
          .pause();
      return function (v) {
        (M.length || c.restart(!0),
          M.push(v.trigger),
          h.push(v),
          i <= M.length && c.progress(1));
      };
    },
    l;
  for (l in e)
    t[l] =
      l.substr(0, 2) === "on" && Te(e[l]) && l !== "onRefreshInit"
        ? u(l, e[l])
        : e[l];
  return (
    Te(i) &&
      ((i = i()),
      de(R, "refresh", function () {
        return (i = e.batchMax());
      })),
    Pr(o).forEach(function (p) {
      var x = {};
      for (l in t) x[l] = t[l];
      ((x.trigger = p), n.push(R.create(x)));
    }),
    n
  );
};
var jn = function (e, n, t, r) {
    return (
      n > r ? e(r) : n < 0 && e(0),
      t > r ? (r - n) / (t - n) : t < 0 ? n / (n - t) : 1
    );
  },
  dn = function o(e, n) {
    (n === !0
      ? e.style.removeProperty("touch-action")
      : (e.style.touchAction =
          n === !0
            ? "auto"
            : n
              ? "pan-" + n + (q.isTouch ? " pinch-zoom" : "")
              : "none"),
      e === qe && o(L, n));
  },
  Vr = { auto: 1, scroll: 1 },
  Yi = function (e) {
    var n = e.event,
      t = e.target,
      r = e.axis,
      i = (n.changedTouches ? n.changedTouches[0] : n).target,
      u = i._gsap || d.core.getCache(i),
      l = Se(),
      p;
    if (!u._isScrollT || l - u._isScrollT > 2e3) {
      for (
        ;
        i &&
        i !== L &&
        ((i.scrollHeight <= i.clientHeight && i.scrollWidth <= i.clientWidth) ||
          !(Vr[(p = Qe(i)).overflowY] || Vr[p.overflowX]));
      )
        i = i.parentNode;
      ((u._isScroll =
        i &&
        i !== t &&
        !Ut(i) &&
        (Vr[(p = Qe(i)).overflowY] || Vr[p.overflowX])),
        (u._isScrollT = l));
    }
    (u._isScroll || r === "x") && (n.stopPropagation(), (n._gsapAllow = !0));
  },
  mi = function (e, n, t, r) {
    return q.create({
      target: e,
      capture: !0,
      debounce: !1,
      lockAxis: !0,
      type: n,
      onWheel: (r = r && Yi),
      onPress: r,
      onDrag: r,
      onScroll: r,
      onEnable: function () {
        return t && de(B, q.eventTypes[0], ti, !1, !0);
      },
      onDisable: function () {
        return pe(B, q.eventTypes[0], ti, !0);
      },
    });
  },
  Fi = /(input|label|select|textarea)/i,
  ei,
  ti = function (e) {
    var n = Fi.test(e.target.tagName);
    (n || ei) && ((e._gsapAllow = !0), (ei = n));
  },
  Li = function (e) {
    (Bt(e) || (e = {}),
      (e.preventDefault = e.isNormalizer = e.allowClicks = !0),
      e.type || (e.type = "wheel,touch"),
      (e.debounce = !!e.debounce),
      (e.id = e.id || "normalizer"));
    var n = e,
      t = n.normalizeScrollX,
      r = n.momentum,
      i = n.allowNestedScroll,
      u = n.onRelease,
      l,
      p,
      x = Ye(e.target) || qe,
      Y = d.core.globals().ScrollSmoother,
      M = Y && Y.get(),
      h =
        Ot &&
        ((e.content && Ye(e.content)) ||
          (M && e.content !== !1 && !M.smooth() && M.content())),
      c = yt(x, re),
      v = yt(x, we),
      U = 1,
      X =
        (q.isTouch && E.visualViewport
          ? E.visualViewport.scale * E.visualViewport.width
          : E.outerWidth) / E.innerWidth,
      ie = 0,
      W = Te(r)
        ? function () {
            return r(l);
          }
        : function () {
            return r || 2.8;
          },
      ge,
      T,
      Ke = mi(x, e.type, !0, i),
      N = function () {
        return (T = !1);
      },
      b = ct,
      ke = ct,
      Ie = function () {
        ((p = ft(x, re)),
          (ke = br(Ot ? 1 : 0, p)),
          t && (b = br(0, ft(x, we))),
          (ge = Gt));
      },
      g = function () {
        ((h._gsap.y = mr(parseFloat(h._gsap.y) + c.offset) + "px"),
          (h.style.transform =
            "matrix3d(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, " +
            parseFloat(h._gsap.y) +
            ", 0, 1)"),
          (c.offset = c.cacheID = 0));
      },
      ue = function () {
        if (T) {
          requestAnimationFrame(N);
          var Q = mr(l.deltaY / 2),
            j = ke(c.v - Q);
          if (h && j !== c.v + c.offset) {
            c.offset = j - c.v;
            var s = mr((parseFloat(h && h._gsap.y) || 0) - c.offset);
            ((h.style.transform =
              "matrix3d(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, " +
              s +
              ", 0, 1)"),
              (h._gsap.y = s + "px"),
              (c.cacheID = k.cache),
              Ct());
          }
          return !0;
        }
        (c.offset && g(), (T = !0));
      },
      y,
      pt,
      oe,
      Pe,
      Ee = function () {
        (Ie(),
          y.isActive() &&
            y.vars.scrollY > p &&
            (c() > p ? y.progress(1) && c(p) : y.resetTo("scrollY", p)));
      };
    return (
      h && d.set(h, { y: "+=0" }),
      (e.ignoreCheck = function (z) {
        return (
          (Ot && z.type === "touchmove" && ue(z)) ||
          (U > 1.05 && z.type !== "touchstart") ||
          l.isGesturing ||
          (z.touches && z.touches.length > 1)
        );
      }),
      (e.onPress = function () {
        T = !1;
        var z = U;
        ((U = mr(((E.visualViewport && E.visualViewport.scale) || 1) / X)),
          y.pause(),
          z !== U && dn(x, U > 1.01 ? !0 : t ? !1 : "x"),
          (pt = v()),
          (oe = c()),
          Ie(),
          (ge = Gt));
      }),
      (e.onRelease = e.onGestureStart =
        function (z, Q) {
          if ((c.offset && g(), !Q)) Pe.restart(!0);
          else {
            k.cache++;
            var j = W(),
              s,
              se;
            (t &&
              ((s = v()),
              (se = s + (j * 0.05 * -z.velocityX) / 0.227),
              (j *= jn(v, s, se, ft(x, we))),
              (y.vars.scrollX = b(se))),
              (s = c()),
              (se = s + (j * 0.05 * -z.velocityY) / 0.227),
              (j *= jn(c, s, se, ft(x, re))),
              (y.vars.scrollY = ke(se)),
              y.invalidate().duration(j).play(0.01),
              ((Ot && y.vars.scrollY >= p) || s >= p - 1) &&
                d.to({}, { onUpdate: Ee, duration: j }));
          }
          u && u(z);
        }),
      (e.onWheel = function () {
        (y._ts && y.pause(), Se() - ie > 1e3 && ((ge = 0), (ie = Se())));
      }),
      (e.onChange = function (z, Q, j, s, se) {
        if (
          (Gt !== ge && Ie(),
          Q && t && v(b(s[2] === Q ? pt + (z.startX - z.x) : v() + Q - s[1])),
          j)
        ) {
          c.offset && g();
          var At = se[2] === j,
            St = At ? oe + z.startY - z.y : c() + j - se[1],
            Ze = ke(St);
          (At && St !== Ze && (oe += Ze - St), c(Ze));
        }
        (j || Q) && Ct();
      }),
      (e.onEnable = function () {
        (dn(x, t ? !1 : "x"),
          R.addEventListener("refresh", Ee),
          de(E, "resize", Ee),
          c.smooth &&
            ((c.target.style.scrollBehavior = "auto"),
            (c.smooth = v.smooth = !1)),
          Ke.enable());
      }),
      (e.onDisable = function () {
        (dn(x, !0),
          pe(E, "resize", Ee),
          R.removeEventListener("refresh", Ee),
          Ke.kill());
      }),
      (e.lockAxis = e.lockAxis !== !1),
      (l = new q(e)),
      (l.iOS = Ot),
      Ot && !c() && c(1),
      Ot && d.ticker.add(ct),
      (Pe = l._dc),
      (y = d.to(l, {
        ease: "power4",
        paused: !0,
        inherit: !1,
        scrollX: t ? "+=0.1" : "+=0",
        scrollY: "+=0.1",
        modifiers: {
          scrollY: vi(c, c(), function () {
            return y.pause();
          }),
        },
        onUpdate: Ct,
        onComplete: Pe.vars.onComplete,
      })),
      l
    );
  };
R.sort = function (o) {
  if (Te(o)) return P.sort(o);
  var e = E.pageYOffset || 0;
  return (
    R.getAll().forEach(function (n) {
      return (n._sortY = n.trigger
        ? e + n.trigger.getBoundingClientRect().top
        : n.start + E.innerHeight);
    }),
    P.sort(
      o ||
        function (n, t) {
          return (
            (n.vars.refreshPriority || 0) * -1e6 +
            (n.vars.containerAnimation ? 1e6 : n._sortY) -
            ((t.vars.containerAnimation ? 1e6 : t._sortY) +
              (t.vars.refreshPriority || 0) * -1e6)
          );
        },
    )
  );
};
R.observe = function (o) {
  return new q(o);
};
R.normalizeScroll = function (o) {
  if (typeof o > "u") return Fe;
  if (o === !0 && Fe) return Fe.enable();
  if (o === !1) {
    (Fe && Fe.kill(), (Fe = o));
    return;
  }
  var e = o instanceof q ? o : Li(o);
  return (
    Fe && Fe.target === e.target && Fe.kill(),
    Ut(e.target) && (Fe = e),
    e
  );
};
R.core = {
  _getVelocityProp: Lr,
  _inputObserver: mi,
  _scrollers: k,
  _proxies: st,
  bridge: {
    ss: function () {
      (je || qt("scrollStart"), (je = Se()));
    },
    ref: function () {
      return Ce;
    },
  },
};
li() && d.registerPlugin(R);
export { R as ScrollTrigger, R as default };
/**i18n:e2f94bf06bdfc8c8ab493a12299261c375fc525ae09e041ca331cb13279050ab*/
