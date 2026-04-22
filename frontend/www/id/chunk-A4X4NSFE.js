import { a as D, b as j, g as Rn } from "./chunk-ZBVOF6Q3.js";
function ui(e, t) {
  return Object.is(e, t);
}
var ee = null,
  ci = !1,
  $a = 1,
  re = Symbol("SIGNAL");
function A(e) {
  let t = ee;
  return ((ee = e), t);
}
function Ha() {
  return ee;
}
var Xt = {
  version: 0,
  lastCleanEpoch: 0,
  dirty: !1,
  producerNode: void 0,
  producerLastReadVersion: void 0,
  producerIndexOfThis: void 0,
  nextProducerIndex: 0,
  liveConsumerNode: void 0,
  liveConsumerIndexOfThis: void 0,
  consumerAllowSignalWrites: !1,
  consumerIsAlwaysLive: !1,
  kind: "unknown",
  producerMustRecompute: () => !1,
  producerRecomputeValue: () => {},
  consumerMarkedDirty: () => {},
  consumerOnSignalRead: () => {},
};
function Ot(e) {
  if (ci) throw new Error("");
  if (ee === null) return;
  ee.consumerOnSignalRead(e);
  let t = ee.nextProducerIndex++;
  if (
    (di(ee), t < ee.producerNode.length && ee.producerNode[t] !== e && zr(ee))
  ) {
    let n = ee.producerNode[t];
    li(n, ee.producerIndexOfThis[t]);
  }
  (ee.producerNode[t] !== e &&
    ((ee.producerNode[t] = e),
    (ee.producerIndexOfThis[t] = zr(ee) ? If(e, ee, t) : 0)),
    (ee.producerLastReadVersion[t] = e.version));
}
function Ef() {
  $a++;
}
function On(e) {
  if (!(zr(e) && !e.dirty) && !(!e.dirty && e.lastCleanEpoch === $a)) {
    if (!e.producerMustRecompute(e) && !qr(e)) {
      Hr(e);
      return;
    }
    (e.producerRecomputeValue(e), Hr(e));
  }
}
function za(e) {
  if (e.liveConsumerNode === void 0) return;
  let t = ci;
  ci = !0;
  try {
    for (let n of e.liveConsumerNode) n.dirty || DD(n);
  } finally {
    ci = t;
  }
}
function qa() {
  return ee?.consumerAllowSignalWrites !== !1;
}
function DD(e) {
  ((e.dirty = !0), za(e), e.consumerMarkedDirty?.(e));
}
function Hr(e) {
  ((e.dirty = !1), (e.lastCleanEpoch = $a));
}
function en(e) {
  return (e && (e.nextProducerIndex = 0), A(e));
}
function kn(e, t) {
  if (
    (A(t),
    !(
      !e ||
      e.producerNode === void 0 ||
      e.producerIndexOfThis === void 0 ||
      e.producerLastReadVersion === void 0
    ))
  ) {
    if (zr(e))
      for (let n = e.nextProducerIndex; n < e.producerNode.length; n++)
        li(e.producerNode[n], e.producerIndexOfThis[n]);
    for (; e.producerNode.length > e.nextProducerIndex; )
      (e.producerNode.pop(),
        e.producerLastReadVersion.pop(),
        e.producerIndexOfThis.pop());
  }
}
function qr(e) {
  di(e);
  for (let t = 0; t < e.producerNode.length; t++) {
    let n = e.producerNode[t],
      r = e.producerLastReadVersion[t];
    if (r !== n.version || (On(n), r !== n.version)) return !0;
  }
  return !1;
}
function Gr(e) {
  if ((di(e), zr(e)))
    for (let t = 0; t < e.producerNode.length; t++)
      li(e.producerNode[t], e.producerIndexOfThis[t]);
  ((e.producerNode.length =
    e.producerLastReadVersion.length =
    e.producerIndexOfThis.length =
      0),
    e.liveConsumerNode &&
      (e.liveConsumerNode.length = e.liveConsumerIndexOfThis.length = 0));
}
function If(e, t, n) {
  if ((Cf(e), e.liveConsumerNode.length === 0 && bf(e)))
    for (let r = 0; r < e.producerNode.length; r++)
      e.producerIndexOfThis[r] = If(e.producerNode[r], e, r);
  return (e.liveConsumerIndexOfThis.push(n), e.liveConsumerNode.push(t) - 1);
}
function li(e, t) {
  if ((Cf(e), e.liveConsumerNode.length === 1 && bf(e)))
    for (let r = 0; r < e.producerNode.length; r++)
      li(e.producerNode[r], e.producerIndexOfThis[r]);
  let n = e.liveConsumerNode.length - 1;
  if (
    ((e.liveConsumerNode[t] = e.liveConsumerNode[n]),
    (e.liveConsumerIndexOfThis[t] = e.liveConsumerIndexOfThis[n]),
    e.liveConsumerNode.length--,
    e.liveConsumerIndexOfThis.length--,
    t < e.liveConsumerNode.length)
  ) {
    let r = e.liveConsumerIndexOfThis[t],
      o = e.liveConsumerNode[t];
    (di(o), (o.producerIndexOfThis[r] = t));
  }
}
function zr(e) {
  return e.consumerIsAlwaysLive || (e?.liveConsumerNode?.length ?? 0) > 0;
}
function di(e) {
  ((e.producerNode ??= []),
    (e.producerIndexOfThis ??= []),
    (e.producerLastReadVersion ??= []));
}
function Cf(e) {
  ((e.liveConsumerNode ??= []), (e.liveConsumerIndexOfThis ??= []));
}
function bf(e) {
  return e.producerNode !== void 0;
}
function fi(e, t) {
  let n = Object.create(wD);
  ((n.computation = e), t !== void 0 && (n.equal = t));
  let r = () => {
    if ((On(n), Ot(n), n.value === At)) throw n.error;
    return n.value;
  };
  return ((r[re] = n), r);
}
var Jt = Symbol("UNSET"),
  An = Symbol("COMPUTING"),
  At = Symbol("ERRORED"),
  wD = j(D({}, Xt), {
    value: Jt,
    dirty: !0,
    error: null,
    equal: ui,
    kind: "computed",
    producerMustRecompute(e) {
      return e.value === Jt || e.value === An;
    },
    producerRecomputeValue(e) {
      if (e.value === An) throw new Error("Detected cycle in computations.");
      let t = e.value;
      e.value = An;
      let n = en(e),
        r,
        o = !1;
      try {
        ((r = e.computation()),
          A(null),
          (o = t !== Jt && t !== At && r !== At && e.equal(t, r)));
      } catch (i) {
        ((r = At), (e.error = i));
      } finally {
        kn(e, n);
      }
      if (o) {
        e.value = t;
        return;
      }
      ((e.value = r), e.version++);
    },
  });
function ED() {
  throw new Error();
}
var Mf = ED;
function Sf(e) {
  Mf(e);
}
function Ga(e) {
  Mf = e;
}
var ID = null;
function Wa(e, t) {
  let n = Object.create(Wr);
  ((n.value = e), t !== void 0 && (n.equal = t));
  let r = () => (Ot(n), n.value);
  return ((r[re] = n), r);
}
function tn(e, t) {
  (qa() || Sf(e), e.equal(e.value, t) || ((e.value = t), CD(e)));
}
function hi(e, t) {
  (qa() || Sf(e), tn(e, t(e.value)));
}
var Wr = j(D({}, Xt), { equal: ui, value: void 0, kind: "signal" });
function CD(e) {
  (e.version++, Ef(), za(e), ID?.());
}
function pi(e, t, n) {
  let r = Object.create(bD);
  ((r.source = e), (r.computation = t), n != null && (r.equal = n));
  let i = () => {
    if ((On(r), Ot(r), r.value === At)) throw r.error;
    return r.value;
  };
  return ((i[re] = r), i);
}
function Za(e, t) {
  (On(e), tn(e, t), Hr(e));
}
function Ya(e, t) {
  (On(e), hi(e, t), Hr(e));
}
var bD = j(D({}, Xt), {
  value: Jt,
  dirty: !0,
  error: null,
  equal: ui,
  kind: "linkedSignal",
  producerMustRecompute(e) {
    return e.value === Jt || e.value === An;
  },
  producerRecomputeValue(e) {
    if (e.value === An) throw new Error("Detected cycle in computations.");
    let t = e.value;
    e.value = An;
    let n = en(e),
      r;
    try {
      let o = e.source(),
        i = t === Jt || t === At ? void 0 : { source: e.sourceValue, value: t };
      ((r = e.computation(o, i)), (e.sourceValue = o));
    } catch (o) {
      ((r = At), (e.error = o));
    } finally {
      kn(e, n);
    }
    if (t !== Jt && r !== At && e.equal(t, r)) {
      e.value = t;
      return;
    }
    ((e.value = r), e.version++);
  },
});
function Qa(e) {
  let t = A(null);
  try {
    return e();
  } finally {
    A(t);
  }
}
var Ka;
function Zr() {
  return Ka;
}
function at(e) {
  let t = Ka;
  return ((Ka = e), t);
}
var gi = Symbol("NotFound");
function M(e) {
  return typeof e == "function";
}
function Pn(e) {
  let n = e((r) => {
    (Error.call(r), (r.stack = new Error().stack));
  });
  return (
    (n.prototype = Object.create(Error.prototype)),
    (n.prototype.constructor = n),
    n
  );
}
var mi = Pn(
  (e) =>
    function (n) {
      (e(this),
        (this.message = n
          ? `${n.length} errors occurred during unsubscription:
${n.map((r, o) => `${o + 1}) ${r.toString()}`).join(`
  `)}`
          : ""),
        (this.name = "UnsubscriptionError"),
        (this.errors = n));
    },
);
function nn(e, t) {
  if (e) {
    let n = e.indexOf(t);
    0 <= n && e.splice(n, 1);
  }
}
var K = class e {
  constructor(t) {
    ((this.initialTeardown = t),
      (this.closed = !1),
      (this._parentage = null),
      (this._finalizers = null));
  }
  unsubscribe() {
    let t;
    if (!this.closed) {
      this.closed = !0;
      let { _parentage: n } = this;
      if (n)
        if (((this._parentage = null), Array.isArray(n)))
          for (let i of n) i.remove(this);
        else n.remove(this);
      let { initialTeardown: r } = this;
      if (M(r))
        try {
          r();
        } catch (i) {
          t = i instanceof mi ? i.errors : [i];
        }
      let { _finalizers: o } = this;
      if (o) {
        this._finalizers = null;
        for (let i of o)
          try {
            Tf(i);
          } catch (s) {
            ((t = t ?? []),
              s instanceof mi ? (t = [...t, ...s.errors]) : t.push(s));
          }
      }
      if (t) throw new mi(t);
    }
  }
  add(t) {
    var n;
    if (t && t !== this)
      if (this.closed) Tf(t);
      else {
        if (t instanceof e) {
          if (t.closed || t._hasParent(this)) return;
          t._addParent(this);
        }
        (this._finalizers =
          (n = this._finalizers) !== null && n !== void 0 ? n : []).push(t);
      }
  }
  _hasParent(t) {
    let { _parentage: n } = this;
    return n === t || (Array.isArray(n) && n.includes(t));
  }
  _addParent(t) {
    let { _parentage: n } = this;
    this._parentage = Array.isArray(n) ? (n.push(t), n) : n ? [n, t] : t;
  }
  _removeParent(t) {
    let { _parentage: n } = this;
    n === t ? (this._parentage = null) : Array.isArray(n) && nn(n, t);
  }
  remove(t) {
    let { _finalizers: n } = this;
    (n && nn(n, t), t instanceof e && t._removeParent(this));
  }
};
K.EMPTY = (() => {
  let e = new K();
  return ((e.closed = !0), e);
})();
var Ja = K.EMPTY;
function yi(e) {
  return (
    e instanceof K ||
    (e && "closed" in e && M(e.remove) && M(e.add) && M(e.unsubscribe))
  );
}
function Tf(e) {
  M(e) ? e() : e.unsubscribe();
}
var Le = {
  onUnhandledError: null,
  onStoppedNotification: null,
  Promise: void 0,
  useDeprecatedSynchronousErrorHandling: !1,
  useDeprecatedNextContext: !1,
};
var Fn = {
  setTimeout(e, t, ...n) {
    let { delegate: r } = Fn;
    return r?.setTimeout ? r.setTimeout(e, t, ...n) : setTimeout(e, t, ...n);
  },
  clearTimeout(e) {
    let { delegate: t } = Fn;
    return (t?.clearTimeout || clearTimeout)(e);
  },
  delegate: void 0,
};
function vi(e) {
  Fn.setTimeout(() => {
    let { onUnhandledError: t } = Le;
    if (t) t(e);
    else throw e;
  });
}
function ct() {}
var _f = Xa("C", void 0, void 0);
function xf(e) {
  return Xa("E", void 0, e);
}
function Nf(e) {
  return Xa("N", e, void 0);
}
function Xa(e, t, n) {
  return { kind: e, value: t, error: n };
}
var rn = null;
function Ln(e) {
  if (Le.useDeprecatedSynchronousErrorHandling) {
    let t = !rn;
    if ((t && (rn = { errorThrown: !1, error: null }), e(), t)) {
      let { errorThrown: n, error: r } = rn;
      if (((rn = null), n)) throw r;
    }
  } else e();
}
function Rf(e) {
  Le.useDeprecatedSynchronousErrorHandling &&
    rn &&
    ((rn.errorThrown = !0), (rn.error = e));
}
var on = class extends K {
    constructor(t) {
      (super(),
        (this.isStopped = !1),
        t
          ? ((this.destination = t), yi(t) && t.add(this))
          : (this.destination = TD));
    }
    static create(t, n, r) {
      return new kt(t, n, r);
    }
    next(t) {
      this.isStopped ? tc(Nf(t), this) : this._next(t);
    }
    error(t) {
      this.isStopped
        ? tc(xf(t), this)
        : ((this.isStopped = !0), this._error(t));
    }
    complete() {
      this.isStopped ? tc(_f, this) : ((this.isStopped = !0), this._complete());
    }
    unsubscribe() {
      this.closed ||
        ((this.isStopped = !0), super.unsubscribe(), (this.destination = null));
    }
    _next(t) {
      this.destination.next(t);
    }
    _error(t) {
      try {
        this.destination.error(t);
      } finally {
        this.unsubscribe();
      }
    }
    _complete() {
      try {
        this.destination.complete();
      } finally {
        this.unsubscribe();
      }
    }
  },
  MD = Function.prototype.bind;
function ec(e, t) {
  return MD.call(e, t);
}
var nc = class {
    constructor(t) {
      this.partialObserver = t;
    }
    next(t) {
      let { partialObserver: n } = this;
      if (n.next)
        try {
          n.next(t);
        } catch (r) {
          Di(r);
        }
    }
    error(t) {
      let { partialObserver: n } = this;
      if (n.error)
        try {
          n.error(t);
        } catch (r) {
          Di(r);
        }
      else Di(t);
    }
    complete() {
      let { partialObserver: t } = this;
      if (t.complete)
        try {
          t.complete();
        } catch (n) {
          Di(n);
        }
    }
  },
  kt = class extends on {
    constructor(t, n, r) {
      super();
      let o;
      if (M(t) || !t)
        o = { next: t ?? void 0, error: n ?? void 0, complete: r ?? void 0 };
      else {
        let i;
        this && Le.useDeprecatedNextContext
          ? ((i = Object.create(t)),
            (i.unsubscribe = () => this.unsubscribe()),
            (o = {
              next: t.next && ec(t.next, i),
              error: t.error && ec(t.error, i),
              complete: t.complete && ec(t.complete, i),
            }))
          : (o = t);
      }
      this.destination = new nc(o);
    }
  };
function Di(e) {
  Le.useDeprecatedSynchronousErrorHandling ? Rf(e) : vi(e);
}
function SD(e) {
  throw e;
}
function tc(e, t) {
  let { onStoppedNotification: n } = Le;
  n && Fn.setTimeout(() => n(e, t));
}
var TD = { closed: !0, next: ct, error: SD, complete: ct };
var jn = (typeof Symbol == "function" && Symbol.observable) || "@@observable";
function pe(e) {
  return e;
}
function rc(...e) {
  return oc(e);
}
function oc(e) {
  return e.length === 0
    ? pe
    : e.length === 1
      ? e[0]
      : function (n) {
          return e.reduce((r, o) => o(r), n);
        };
}
var F = (() => {
  class e {
    constructor(n) {
      n && (this._subscribe = n);
    }
    lift(n) {
      let r = new e();
      return ((r.source = this), (r.operator = n), r);
    }
    subscribe(n, r, o) {
      let i = xD(n) ? n : new kt(n, r, o);
      return (
        Ln(() => {
          let { operator: s, source: a } = this;
          i.add(
            s ? s.call(i, a) : a ? this._subscribe(i) : this._trySubscribe(i),
          );
        }),
        i
      );
    }
    _trySubscribe(n) {
      try {
        return this._subscribe(n);
      } catch (r) {
        n.error(r);
      }
    }
    forEach(n, r) {
      return (
        (r = Af(r)),
        new r((o, i) => {
          let s = new kt({
            next: (a) => {
              try {
                n(a);
              } catch (c) {
                (i(c), s.unsubscribe());
              }
            },
            error: i,
            complete: o,
          });
          this.subscribe(s);
        })
      );
    }
    _subscribe(n) {
      var r;
      return (r = this.source) === null || r === void 0
        ? void 0
        : r.subscribe(n);
    }
    [jn]() {
      return this;
    }
    pipe(...n) {
      return oc(n)(this);
    }
    toPromise(n) {
      return (
        (n = Af(n)),
        new n((r, o) => {
          let i;
          this.subscribe(
            (s) => (i = s),
            (s) => o(s),
            () => r(i),
          );
        })
      );
    }
  }
  return ((e.create = (t) => new e(t)), e);
})();
function Af(e) {
  var t;
  return (t = e ?? Le.Promise) !== null && t !== void 0 ? t : Promise;
}
function _D(e) {
  return e && M(e.next) && M(e.error) && M(e.complete);
}
function xD(e) {
  return (e && e instanceof on) || (_D(e) && yi(e));
}
function ic(e) {
  return M(e?.lift);
}
function x(e) {
  return (t) => {
    if (ic(t))
      return t.lift(function (n) {
        try {
          return e(n, this);
        } catch (r) {
          this.error(r);
        }
      });
    throw new TypeError("Unable to lift unknown Observable type");
  };
}
function T(e, t, n, r, o) {
  return new sc(e, t, n, r, o);
}
var sc = class extends on {
  constructor(t, n, r, o, i, s) {
    (super(t),
      (this.onFinalize = i),
      (this.shouldUnsubscribe = s),
      (this._next = n
        ? function (a) {
            try {
              n(a);
            } catch (c) {
              t.error(c);
            }
          }
        : super._next),
      (this._error = o
        ? function (a) {
            try {
              o(a);
            } catch (c) {
              t.error(c);
            } finally {
              this.unsubscribe();
            }
          }
        : super._error),
      (this._complete = r
        ? function () {
            try {
              r();
            } catch (a) {
              t.error(a);
            } finally {
              this.unsubscribe();
            }
          }
        : super._complete));
  }
  unsubscribe() {
    var t;
    if (!this.shouldUnsubscribe || this.shouldUnsubscribe()) {
      let { closed: n } = this;
      (super.unsubscribe(),
        !n && ((t = this.onFinalize) === null || t === void 0 || t.call(this)));
    }
  }
};
function Vn() {
  return x((e, t) => {
    let n = null;
    e._refCount++;
    let r = T(t, void 0, void 0, void 0, () => {
      if (!e || e._refCount <= 0 || 0 < --e._refCount) {
        n = null;
        return;
      }
      let o = e._connection,
        i = n;
      ((n = null), o && (!i || o === i) && o.unsubscribe(), t.unsubscribe());
    });
    (e.subscribe(r), r.closed || (n = e.connect()));
  });
}
var Bn = class extends F {
  constructor(t, n) {
    (super(),
      (this.source = t),
      (this.subjectFactory = n),
      (this._subject = null),
      (this._refCount = 0),
      (this._connection = null),
      ic(t) && (this.lift = t.lift));
  }
  _subscribe(t) {
    return this.getSubject().subscribe(t);
  }
  getSubject() {
    let t = this._subject;
    return (
      (!t || t.isStopped) && (this._subject = this.subjectFactory()),
      this._subject
    );
  }
  _teardown() {
    this._refCount = 0;
    let { _connection: t } = this;
    ((this._subject = this._connection = null), t?.unsubscribe());
  }
  connect() {
    let t = this._connection;
    if (!t) {
      t = this._connection = new K();
      let n = this.getSubject();
      (t.add(
        this.source.subscribe(
          T(
            n,
            void 0,
            () => {
              (this._teardown(), n.complete());
            },
            (r) => {
              (this._teardown(), n.error(r));
            },
            () => this._teardown(),
          ),
        ),
      ),
        t.closed && ((this._connection = null), (t = K.EMPTY)));
    }
    return t;
  }
  refCount() {
    return Vn()(this);
  }
};
var Of = Pn(
  (e) =>
    function () {
      (e(this),
        (this.name = "ObjectUnsubscribedError"),
        (this.message = "object unsubscribed"));
    },
);
var X = (() => {
    class e extends F {
      constructor() {
        (super(),
          (this.closed = !1),
          (this.currentObservers = null),
          (this.observers = []),
          (this.isStopped = !1),
          (this.hasError = !1),
          (this.thrownError = null));
      }
      lift(n) {
        let r = new wi(this, this);
        return ((r.operator = n), r);
      }
      _throwIfClosed() {
        if (this.closed) throw new Of();
      }
      next(n) {
        Ln(() => {
          if ((this._throwIfClosed(), !this.isStopped)) {
            this.currentObservers ||
              (this.currentObservers = Array.from(this.observers));
            for (let r of this.currentObservers) r.next(n);
          }
        });
      }
      error(n) {
        Ln(() => {
          if ((this._throwIfClosed(), !this.isStopped)) {
            ((this.hasError = this.isStopped = !0), (this.thrownError = n));
            let { observers: r } = this;
            for (; r.length; ) r.shift().error(n);
          }
        });
      }
      complete() {
        Ln(() => {
          if ((this._throwIfClosed(), !this.isStopped)) {
            this.isStopped = !0;
            let { observers: n } = this;
            for (; n.length; ) n.shift().complete();
          }
        });
      }
      unsubscribe() {
        ((this.isStopped = this.closed = !0),
          (this.observers = this.currentObservers = null));
      }
      get observed() {
        var n;
        return (
          ((n = this.observers) === null || n === void 0 ? void 0 : n.length) >
          0
        );
      }
      _trySubscribe(n) {
        return (this._throwIfClosed(), super._trySubscribe(n));
      }
      _subscribe(n) {
        return (
          this._throwIfClosed(),
          this._checkFinalizedStatuses(n),
          this._innerSubscribe(n)
        );
      }
      _innerSubscribe(n) {
        let { hasError: r, isStopped: o, observers: i } = this;
        return r || o
          ? Ja
          : ((this.currentObservers = null),
            i.push(n),
            new K(() => {
              ((this.currentObservers = null), nn(i, n));
            }));
      }
      _checkFinalizedStatuses(n) {
        let { hasError: r, thrownError: o, isStopped: i } = this;
        r ? n.error(o) : i && n.complete();
      }
      asObservable() {
        let n = new F();
        return ((n.source = this), n);
      }
    }
    return ((e.create = (t, n) => new wi(t, n)), e);
  })(),
  wi = class extends X {
    constructor(t, n) {
      (super(), (this.destination = t), (this.source = n));
    }
    next(t) {
      var n, r;
      (r =
        (n = this.destination) === null || n === void 0 ? void 0 : n.next) ===
        null ||
        r === void 0 ||
        r.call(n, t);
    }
    error(t) {
      var n, r;
      (r =
        (n = this.destination) === null || n === void 0 ? void 0 : n.error) ===
        null ||
        r === void 0 ||
        r.call(n, t);
    }
    complete() {
      var t, n;
      (n =
        (t = this.destination) === null || t === void 0
          ? void 0
          : t.complete) === null ||
        n === void 0 ||
        n.call(t);
    }
    _subscribe(t) {
      var n, r;
      return (r =
        (n = this.source) === null || n === void 0
          ? void 0
          : n.subscribe(t)) !== null && r !== void 0
        ? r
        : Ja;
    }
  };
var se = class extends X {
  constructor(t) {
    (super(), (this._value = t));
  }
  get value() {
    return this.getValue();
  }
  _subscribe(t) {
    let n = super._subscribe(t);
    return (!n.closed && t.next(this._value), n);
  }
  getValue() {
    let { hasError: t, thrownError: n, _value: r } = this;
    if (t) throw n;
    return (this._throwIfClosed(), r);
  }
  next(t) {
    super.next((this._value = t));
  }
};
var Yr = {
  now() {
    return (Yr.delegate || Date).now();
  },
  delegate: void 0,
};
var ac = class extends X {
  constructor(t = 1 / 0, n = 1 / 0, r = Yr) {
    (super(),
      (this._bufferSize = t),
      (this._windowTime = n),
      (this._timestampProvider = r),
      (this._buffer = []),
      (this._infiniteTimeWindow = !0),
      (this._infiniteTimeWindow = n === 1 / 0),
      (this._bufferSize = Math.max(1, t)),
      (this._windowTime = Math.max(1, n)));
  }
  next(t) {
    let {
      isStopped: n,
      _buffer: r,
      _infiniteTimeWindow: o,
      _timestampProvider: i,
      _windowTime: s,
    } = this;
    (n || (r.push(t), !o && r.push(i.now() + s)),
      this._trimBuffer(),
      super.next(t));
  }
  _subscribe(t) {
    (this._throwIfClosed(), this._trimBuffer());
    let n = this._innerSubscribe(t),
      { _infiniteTimeWindow: r, _buffer: o } = this,
      i = o.slice();
    for (let s = 0; s < i.length && !t.closed; s += r ? 1 : 2) t.next(i[s]);
    return (this._checkFinalizedStatuses(t), n);
  }
  _trimBuffer() {
    let {
        _bufferSize: t,
        _timestampProvider: n,
        _buffer: r,
        _infiniteTimeWindow: o,
      } = this,
      i = (o ? 1 : 2) * t;
    if ((t < 1 / 0 && i < r.length && r.splice(0, r.length - i), !o)) {
      let s = n.now(),
        a = 0;
      for (let c = 1; c < r.length && r[c] <= s; c += 2) a = c;
      a && r.splice(0, a + 1);
    }
  }
};
var Ei = class extends K {
  constructor(t, n) {
    super();
  }
  schedule(t, n = 0) {
    return this;
  }
};
var Qr = {
  setInterval(e, t, ...n) {
    let { delegate: r } = Qr;
    return r?.setInterval ? r.setInterval(e, t, ...n) : setInterval(e, t, ...n);
  },
  clearInterval(e) {
    let { delegate: t } = Qr;
    return (t?.clearInterval || clearInterval)(e);
  },
  delegate: void 0,
};
var Ii = class extends Ei {
  constructor(t, n) {
    (super(t, n), (this.scheduler = t), (this.work = n), (this.pending = !1));
  }
  schedule(t, n = 0) {
    var r;
    if (this.closed) return this;
    this.state = t;
    let o = this.id,
      i = this.scheduler;
    return (
      o != null && (this.id = this.recycleAsyncId(i, o, n)),
      (this.pending = !0),
      (this.delay = n),
      (this.id =
        (r = this.id) !== null && r !== void 0
          ? r
          : this.requestAsyncId(i, this.id, n)),
      this
    );
  }
  requestAsyncId(t, n, r = 0) {
    return Qr.setInterval(t.flush.bind(t, this), r);
  }
  recycleAsyncId(t, n, r = 0) {
    if (r != null && this.delay === r && this.pending === !1) return n;
    n != null && Qr.clearInterval(n);
  }
  execute(t, n) {
    if (this.closed) return new Error("executing a cancelled action");
    this.pending = !1;
    let r = this._execute(t, n);
    if (r) return r;
    this.pending === !1 &&
      this.id != null &&
      (this.id = this.recycleAsyncId(this.scheduler, this.id, null));
  }
  _execute(t, n) {
    let r = !1,
      o;
    try {
      this.work(t);
    } catch (i) {
      ((r = !0), (o = i || new Error("Scheduled action threw falsy error")));
    }
    if (r) return (this.unsubscribe(), o);
  }
  unsubscribe() {
    if (!this.closed) {
      let { id: t, scheduler: n } = this,
        { actions: r } = n;
      ((this.work = this.state = this.scheduler = null),
        (this.pending = !1),
        nn(r, this),
        t != null && (this.id = this.recycleAsyncId(n, t, null)),
        (this.delay = null),
        super.unsubscribe());
    }
  }
};
var Un = class e {
  constructor(t, n = e.now) {
    ((this.schedulerActionCtor = t), (this.now = n));
  }
  schedule(t, n = 0, r) {
    return new this.schedulerActionCtor(this, t).schedule(r, n);
  }
};
Un.now = Yr.now;
var Ci = class extends Un {
  constructor(t, n = Un.now) {
    (super(t, n), (this.actions = []), (this._active = !1));
  }
  flush(t) {
    let { actions: n } = this;
    if (this._active) {
      n.push(t);
      return;
    }
    let r;
    this._active = !0;
    do if ((r = t.execute(t.state, t.delay))) break;
    while ((t = n.shift()));
    if (((this._active = !1), r)) {
      for (; (t = n.shift()); ) t.unsubscribe();
      throw r;
    }
  }
};
var ut = new Ci(Ii),
  kf = ut;
var le = new F((e) => e.complete());
function bi(e) {
  return e && M(e.schedule);
}
function cc(e) {
  return e[e.length - 1];
}
function $n(e) {
  return M(cc(e)) ? e.pop() : void 0;
}
function ze(e) {
  return bi(cc(e)) ? e.pop() : void 0;
}
function Pf(e, t) {
  return typeof cc(e) == "number" ? e.pop() : t;
}
function Lf(e, t, n, r) {
  function o(i) {
    return i instanceof n
      ? i
      : new n(function (s) {
          s(i);
        });
  }
  return new (n || (n = Promise))(function (i, s) {
    function a(l) {
      try {
        u(r.next(l));
      } catch (d) {
        s(d);
      }
    }
    function c(l) {
      try {
        u(r.throw(l));
      } catch (d) {
        s(d);
      }
    }
    function u(l) {
      l.done ? i(l.value) : o(l.value).then(a, c);
    }
    u((r = r.apply(e, t || [])).next());
  });
}
function Ff(e) {
  var t = typeof Symbol == "function" && Symbol.iterator,
    n = t && e[t],
    r = 0;
  if (n) return n.call(e);
  if (e && typeof e.length == "number")
    return {
      next: function () {
        return (
          e && r >= e.length && (e = void 0),
          { value: e && e[r++], done: !e }
        );
      },
    };
  throw new TypeError(
    t ? "Object is not iterable." : "Symbol.iterator is not defined.",
  );
}
function sn(e) {
  return this instanceof sn ? ((this.v = e), this) : new sn(e);
}
function jf(e, t, n) {
  if (!Symbol.asyncIterator)
    throw new TypeError("Symbol.asyncIterator is not defined.");
  var r = n.apply(e, t || []),
    o,
    i = [];
  return (
    (o = Object.create(
      (typeof AsyncIterator == "function" ? AsyncIterator : Object).prototype,
    )),
    a("next"),
    a("throw"),
    a("return", s),
    (o[Symbol.asyncIterator] = function () {
      return this;
    }),
    o
  );
  function s(f) {
    return function (p) {
      return Promise.resolve(p).then(f, d);
    };
  }
  function a(f, p) {
    r[f] &&
      ((o[f] = function (g) {
        return new Promise(function (y, w) {
          i.push([f, g, y, w]) > 1 || c(f, g);
        });
      }),
      p && (o[f] = p(o[f])));
  }
  function c(f, p) {
    try {
      u(r[f](p));
    } catch (g) {
      h(i[0][3], g);
    }
  }
  function u(f) {
    f.value instanceof sn
      ? Promise.resolve(f.value.v).then(l, d)
      : h(i[0][2], f);
  }
  function l(f) {
    c("next", f);
  }
  function d(f) {
    c("throw", f);
  }
  function h(f, p) {
    (f(p), i.shift(), i.length && c(i[0][0], i[0][1]));
  }
}
function Vf(e) {
  if (!Symbol.asyncIterator)
    throw new TypeError("Symbol.asyncIterator is not defined.");
  var t = e[Symbol.asyncIterator],
    n;
  return t
    ? t.call(e)
    : ((e = typeof Ff == "function" ? Ff(e) : e[Symbol.iterator]()),
      (n = {}),
      r("next"),
      r("throw"),
      r("return"),
      (n[Symbol.asyncIterator] = function () {
        return this;
      }),
      n);
  function r(i) {
    n[i] =
      e[i] &&
      function (s) {
        return new Promise(function (a, c) {
          ((s = e[i](s)), o(a, c, s.done, s.value));
        });
      };
  }
  function o(i, s, a, c) {
    Promise.resolve(c).then(function (u) {
      i({ value: u, done: a });
    }, s);
  }
}
var Hn = (e) => e && typeof e.length == "number" && typeof e != "function";
function Mi(e) {
  return M(e?.then);
}
function Si(e) {
  return M(e[jn]);
}
function Ti(e) {
  return Symbol.asyncIterator && M(e?.[Symbol.asyncIterator]);
}
function _i(e) {
  return new TypeError(
    `You provided ${e !== null && typeof e == "object" ? "an invalid object" : `'${e}'`} where a stream was expected. You can provide an Observable, Promise, ReadableStream, Array, AsyncIterable, or Iterable.`,
  );
}
function ND() {
  return typeof Symbol != "function" || !Symbol.iterator
    ? "@@iterator"
    : Symbol.iterator;
}
var xi = ND();
function Ni(e) {
  return M(e?.[xi]);
}
function Ri(e) {
  return jf(this, arguments, function* () {
    let n = e.getReader();
    try {
      for (;;) {
        let { value: r, done: o } = yield sn(n.read());
        if (o) return yield sn(void 0);
        yield yield sn(r);
      }
    } finally {
      n.releaseLock();
    }
  });
}
function Ai(e) {
  return M(e?.getReader);
}
function V(e) {
  if (e instanceof F) return e;
  if (e != null) {
    if (Si(e)) return RD(e);
    if (Hn(e)) return AD(e);
    if (Mi(e)) return OD(e);
    if (Ti(e)) return Bf(e);
    if (Ni(e)) return kD(e);
    if (Ai(e)) return PD(e);
  }
  throw _i(e);
}
function RD(e) {
  return new F((t) => {
    let n = e[jn]();
    if (M(n.subscribe)) return n.subscribe(t);
    throw new TypeError(
      "Provided object does not correctly implement Symbol.observable",
    );
  });
}
function AD(e) {
  return new F((t) => {
    for (let n = 0; n < e.length && !t.closed; n++) t.next(e[n]);
    t.complete();
  });
}
function OD(e) {
  return new F((t) => {
    e.then(
      (n) => {
        t.closed || (t.next(n), t.complete());
      },
      (n) => t.error(n),
    ).then(null, vi);
  });
}
function kD(e) {
  return new F((t) => {
    for (let n of e) if ((t.next(n), t.closed)) return;
    t.complete();
  });
}
function Bf(e) {
  return new F((t) => {
    FD(e, t).catch((n) => t.error(n));
  });
}
function PD(e) {
  return Bf(Ri(e));
}
function FD(e, t) {
  var n, r, o, i;
  return Lf(this, void 0, void 0, function* () {
    try {
      for (n = Vf(e); (r = yield n.next()), !r.done; ) {
        let s = r.value;
        if ((t.next(s), t.closed)) return;
      }
    } catch (s) {
      o = { error: s };
    } finally {
      try {
        r && !r.done && (i = n.return) && (yield i.call(n));
      } finally {
        if (o) throw o.error;
      }
    }
    t.complete();
  });
}
function Ee(e, t, n, r = 0, o = !1) {
  let i = t.schedule(function () {
    (n(), o ? e.add(this.schedule(null, r)) : this.unsubscribe());
  }, r);
  if ((e.add(i), !o)) return i;
}
function Oi(e, t = 0) {
  return x((n, r) => {
    n.subscribe(
      T(
        r,
        (o) => Ee(r, e, () => r.next(o), t),
        () => Ee(r, e, () => r.complete(), t),
        (o) => Ee(r, e, () => r.error(o), t),
      ),
    );
  });
}
function ki(e, t = 0) {
  return x((n, r) => {
    r.add(e.schedule(() => n.subscribe(r), t));
  });
}
function Uf(e, t) {
  return V(e).pipe(ki(t), Oi(t));
}
function $f(e, t) {
  return V(e).pipe(ki(t), Oi(t));
}
function Hf(e, t) {
  return new F((n) => {
    let r = 0;
    return t.schedule(function () {
      r === e.length
        ? n.complete()
        : (n.next(e[r++]), n.closed || this.schedule());
    });
  });
}
function zf(e, t) {
  return new F((n) => {
    let r;
    return (
      Ee(n, t, () => {
        ((r = e[xi]()),
          Ee(
            n,
            t,
            () => {
              let o, i;
              try {
                ({ value: o, done: i } = r.next());
              } catch (s) {
                n.error(s);
                return;
              }
              i ? n.complete() : n.next(o);
            },
            0,
            !0,
          ));
      }),
      () => M(r?.return) && r.return()
    );
  });
}
function Pi(e, t) {
  if (!e) throw new Error("Iterable cannot be null");
  return new F((n) => {
    Ee(n, t, () => {
      let r = e[Symbol.asyncIterator]();
      Ee(
        n,
        t,
        () => {
          r.next().then((o) => {
            o.done ? n.complete() : n.next(o.value);
          });
        },
        0,
        !0,
      );
    });
  });
}
function qf(e, t) {
  return Pi(Ri(e), t);
}
function Gf(e, t) {
  if (e != null) {
    if (Si(e)) return Uf(e, t);
    if (Hn(e)) return Hf(e, t);
    if (Mi(e)) return $f(e, t);
    if (Ti(e)) return Pi(e, t);
    if (Ni(e)) return zf(e, t);
    if (Ai(e)) return qf(e, t);
  }
  throw _i(e);
}
function G(e, t) {
  return t ? Gf(e, t) : V(e);
}
function S(...e) {
  let t = ze(e);
  return G(e, t);
}
function zn(e, t) {
  let n = M(e) ? e : () => e,
    r = (o) => o.error(n());
  return new F(t ? (o) => t.schedule(r, 0, o) : r);
}
function uc(e) {
  return !!e && (e instanceof F || (M(e.lift) && M(e.subscribe)));
}
var je = Pn(
  (e) =>
    function () {
      (e(this),
        (this.name = "EmptyError"),
        (this.message = "no elements in sequence"));
    },
);
function LD(e, t) {
  let n = typeof t == "object";
  return new Promise((r, o) => {
    let i = new kt({
      next: (s) => {
        (r(s), i.unsubscribe());
      },
      error: o,
      complete: () => {
        n ? r(t.defaultValue) : o(new je());
      },
    });
    e.subscribe(i);
  });
}
function Wf(e) {
  return e instanceof Date && !isNaN(e);
}
function O(e, t) {
  return x((n, r) => {
    let o = 0;
    n.subscribe(
      T(r, (i) => {
        r.next(e.call(t, i, o++));
      }),
    );
  });
}
var { isArray: jD } = Array;
function VD(e, t) {
  return jD(t) ? e(...t) : e(t);
}
function qn(e) {
  return O((t) => VD(e, t));
}
var { isArray: BD } = Array,
  { getPrototypeOf: UD, prototype: $D, keys: HD } = Object;
function Fi(e) {
  if (e.length === 1) {
    let t = e[0];
    if (BD(t)) return { args: t, keys: null };
    if (zD(t)) {
      let n = HD(t);
      return { args: n.map((r) => t[r]), keys: n };
    }
  }
  return { args: e, keys: null };
}
function zD(e) {
  return e && typeof e == "object" && UD(e) === $D;
}
function Li(e, t) {
  return e.reduce((n, r, o) => ((n[r] = t[o]), n), {});
}
function Kr(...e) {
  let t = ze(e),
    n = $n(e),
    { args: r, keys: o } = Fi(e);
  if (r.length === 0) return G([], t);
  let i = new F(qD(r, t, o ? (s) => Li(o, s) : pe));
  return n ? i.pipe(qn(n)) : i;
}
function qD(e, t, n = pe) {
  return (r) => {
    Zf(
      t,
      () => {
        let { length: o } = e,
          i = new Array(o),
          s = o,
          a = o;
        for (let c = 0; c < o; c++)
          Zf(
            t,
            () => {
              let u = G(e[c], t),
                l = !1;
              u.subscribe(
                T(
                  r,
                  (d) => {
                    ((i[c] = d),
                      l || ((l = !0), a--),
                      a || r.next(n(i.slice())));
                  },
                  () => {
                    --s || r.complete();
                  },
                ),
              );
            },
            r,
          );
      },
      r,
    );
  };
}
function Zf(e, t, n) {
  e ? Ee(n, e, t) : t();
}
function Yf(e, t, n, r, o, i, s, a) {
  let c = [],
    u = 0,
    l = 0,
    d = !1,
    h = () => {
      d && !c.length && !u && t.complete();
    },
    f = (g) => (u < r ? p(g) : c.push(g)),
    p = (g) => {
      (i && t.next(g), u++);
      let y = !1;
      V(n(g, l++)).subscribe(
        T(
          t,
          (w) => {
            (o?.(w), i ? f(w) : t.next(w));
          },
          () => {
            y = !0;
          },
          void 0,
          () => {
            if (y)
              try {
                for (u--; c.length && u < r; ) {
                  let w = c.shift();
                  s ? Ee(t, s, () => p(w)) : p(w);
                }
                h();
              } catch (w) {
                t.error(w);
              }
          },
        ),
      );
    };
  return (
    e.subscribe(
      T(t, f, () => {
        ((d = !0), h());
      }),
    ),
    () => {
      a?.();
    }
  );
}
function Z(e, t, n = 1 / 0) {
  return M(t)
    ? Z((r, o) => O((i, s) => t(r, i, o, s))(V(e(r, o))), n)
    : (typeof t == "number" && (n = t), x((r, o) => Yf(r, o, e, n)));
}
function Pt(e = 1 / 0) {
  return Z(pe, e);
}
function Qf() {
  return Pt(1);
}
function Ft(...e) {
  return Qf()(G(e, ze(e)));
}
function ji(e) {
  return new F((t) => {
    V(e()).subscribe(t);
  });
}
function GD(...e) {
  let t = $n(e),
    { args: n, keys: r } = Fi(e),
    o = new F((i) => {
      let { length: s } = n;
      if (!s) {
        i.complete();
        return;
      }
      let a = new Array(s),
        c = s,
        u = s;
      for (let l = 0; l < s; l++) {
        let d = !1;
        V(n[l]).subscribe(
          T(
            i,
            (h) => {
              (d || ((d = !0), u--), (a[l] = h));
            },
            () => c--,
            void 0,
            () => {
              (!c || !d) && (u || i.next(r ? Li(r, a) : a), i.complete());
            },
          ),
        );
      }
    });
  return t ? o.pipe(qn(t)) : o;
}
var WD = ["addListener", "removeListener"],
  ZD = ["addEventListener", "removeEventListener"],
  YD = ["on", "off"];
function lc(e, t, n, r) {
  if ((M(n) && ((r = n), (n = void 0)), r)) return lc(e, t, n).pipe(qn(r));
  let [o, i] = JD(e)
    ? ZD.map((s) => (a) => e[s](t, a, n))
    : QD(e)
      ? WD.map(Kf(e, t))
      : KD(e)
        ? YD.map(Kf(e, t))
        : [];
  if (!o && Hn(e)) return Z((s) => lc(s, t, n))(V(e));
  if (!o) throw new TypeError("Invalid event target");
  return new F((s) => {
    let a = (...c) => s.next(1 < c.length ? c : c[0]);
    return (o(a), () => i(a));
  });
}
function Kf(e, t) {
  return (n) => (r) => e[n](t, r);
}
function QD(e) {
  return M(e.addListener) && M(e.removeListener);
}
function KD(e) {
  return M(e.on) && M(e.off);
}
function JD(e) {
  return M(e.addEventListener) && M(e.removeEventListener);
}
function Gn(e = 0, t, n = kf) {
  let r = -1;
  return (
    t != null && (bi(t) ? (n = t) : (r = t)),
    new F((o) => {
      let i = Wf(e) ? +e - n.now() : e;
      i < 0 && (i = 0);
      let s = 0;
      return n.schedule(function () {
        o.closed ||
          (o.next(s++), 0 <= r ? this.schedule(void 0, r) : o.complete());
      }, i);
    })
  );
}
function XD(e = 0, t = ut) {
  return (e < 0 && (e = 0), Gn(e, e, t));
}
function ew(...e) {
  let t = ze(e),
    n = Pf(e, 1 / 0),
    r = e;
  return r.length ? (r.length === 1 ? V(r[0]) : Pt(n)(G(r, t))) : le;
}
function ge(e, t) {
  return x((n, r) => {
    let o = 0;
    n.subscribe(T(r, (i) => e.call(t, i, o++) && r.next(i)));
  });
}
function Jf(e) {
  return x((t, n) => {
    let r = !1,
      o = null,
      i = null,
      s = !1,
      a = () => {
        if ((i?.unsubscribe(), (i = null), r)) {
          r = !1;
          let u = o;
          ((o = null), n.next(u));
        }
        s && n.complete();
      },
      c = () => {
        ((i = null), s && n.complete());
      };
    t.subscribe(
      T(
        n,
        (u) => {
          ((r = !0), (o = u), i || V(e(u)).subscribe((i = T(n, a, c))));
        },
        () => {
          ((s = !0), (!r || !i || i.closed) && n.complete());
        },
      ),
    );
  });
}
function tw(e, t = ut) {
  return Jf(() => Gn(e, t));
}
function Lt(e) {
  return x((t, n) => {
    let r = null,
      o = !1,
      i;
    ((r = t.subscribe(
      T(n, void 0, void 0, (s) => {
        ((i = V(e(s, Lt(e)(t)))),
          r ? (r.unsubscribe(), (r = null), i.subscribe(n)) : (o = !0));
      }),
    )),
      o && (r.unsubscribe(), (r = null), i.subscribe(n)));
  });
}
function Xf(e, t, n, r, o) {
  return (i, s) => {
    let a = n,
      c = t,
      u = 0;
    i.subscribe(
      T(
        s,
        (l) => {
          let d = u++;
          ((c = a ? e(c, l, d) : ((a = !0), l)), r && s.next(c));
        },
        o &&
          (() => {
            (a && s.next(c), s.complete());
          }),
      ),
    );
  };
}
function lt(e, t) {
  return M(t) ? Z(e, t, 1) : Z(e, 1);
}
function eh(e, t = ut) {
  return x((n, r) => {
    let o = null,
      i = null,
      s = null,
      a = () => {
        if (o) {
          (o.unsubscribe(), (o = null));
          let u = i;
          ((i = null), r.next(u));
        }
      };
    function c() {
      let u = s + e,
        l = t.now();
      if (l < u) {
        ((o = this.schedule(void 0, u - l)), r.add(o));
        return;
      }
      a();
    }
    n.subscribe(
      T(
        r,
        (u) => {
          ((i = u), (s = t.now()), o || ((o = t.schedule(c, e)), r.add(o)));
        },
        () => {
          (a(), r.complete());
        },
        void 0,
        () => {
          i = o = null;
        },
      ),
    );
  });
}
function jt(e) {
  return x((t, n) => {
    let r = !1;
    t.subscribe(
      T(
        n,
        (o) => {
          ((r = !0), n.next(o));
        },
        () => {
          (r || n.next(e), n.complete());
        },
      ),
    );
  });
}
function Oe(e) {
  return e <= 0
    ? () => le
    : x((t, n) => {
        let r = 0;
        t.subscribe(
          T(n, (o) => {
            ++r <= e && (n.next(o), e <= r && n.complete());
          }),
        );
      });
}
function th() {
  return x((e, t) => {
    e.subscribe(T(t, ct));
  });
}
function nh(e) {
  return O(() => e);
}
function dc(e, t) {
  return t
    ? (n) => Ft(t.pipe(Oe(1), th()), n.pipe(dc(e)))
    : Z((n, r) => V(e(n, r)).pipe(Oe(1), nh(n)));
}
function nw(e, t = ut) {
  let n = Gn(e, t);
  return dc(() => n);
}
function Vi(e = rw) {
  return x((t, n) => {
    let r = !1;
    t.subscribe(
      T(
        n,
        (o) => {
          ((r = !0), n.next(o));
        },
        () => (r ? n.complete() : n.error(e())),
      ),
    );
  });
}
function rw() {
  return new je();
}
function an(e) {
  return x((t, n) => {
    try {
      t.subscribe(n);
    } finally {
      n.add(e);
    }
  });
}
function qe(e, t) {
  let n = arguments.length >= 2;
  return (r) =>
    r.pipe(
      e ? ge((o, i) => e(o, i, r)) : pe,
      Oe(1),
      n ? jt(t) : Vi(() => new je()),
    );
}
function Wn(e) {
  return e <= 0
    ? () => le
    : x((t, n) => {
        let r = [];
        t.subscribe(
          T(
            n,
            (o) => {
              (r.push(o), e < r.length && r.shift());
            },
            () => {
              for (let o of r) n.next(o);
              n.complete();
            },
            void 0,
            () => {
              r = null;
            },
          ),
        );
      });
}
function fc(e, t) {
  let n = arguments.length >= 2;
  return (r) =>
    r.pipe(
      e ? ge((o, i) => e(o, i, r)) : pe,
      Wn(1),
      n ? jt(t) : Vi(() => new je()),
    );
}
function hc(e, t) {
  return x(Xf(e, t, arguments.length >= 2, !0));
}
function Bi(...e) {
  let t = ze(e);
  return x((n, r) => {
    (t ? Ft(e, n, t) : Ft(e, n)).subscribe(r);
  });
}
function me(e, t) {
  return x((n, r) => {
    let o = null,
      i = 0,
      s = !1,
      a = () => s && !o && r.complete();
    n.subscribe(
      T(
        r,
        (c) => {
          o?.unsubscribe();
          let u = 0,
            l = i++;
          V(e(c, l)).subscribe(
            (o = T(
              r,
              (d) => r.next(t ? t(c, d, l, u++) : d),
              () => {
                ((o = null), a());
              },
            )),
          );
        },
        () => {
          ((s = !0), a());
        },
      ),
    );
  });
}
function Ui(e) {
  return x((t, n) => {
    (V(e).subscribe(T(n, () => n.complete(), ct)), !n.closed && t.subscribe(n));
  });
}
function ow(e, t = !1) {
  return x((n, r) => {
    let o = 0;
    n.subscribe(
      T(r, (i) => {
        let s = e(i, o++);
        ((s || t) && r.next(i), !s && r.complete());
      }),
    );
  });
}
function oe(e, t, n) {
  let r = M(e) || t || n ? { next: e, error: t, complete: n } : e;
  return r
    ? x((o, i) => {
        var s;
        (s = r.subscribe) === null || s === void 0 || s.call(r);
        let a = !0;
        o.subscribe(
          T(
            i,
            (c) => {
              var u;
              ((u = r.next) === null || u === void 0 || u.call(r, c),
                i.next(c));
            },
            () => {
              var c;
              ((a = !1),
                (c = r.complete) === null || c === void 0 || c.call(r),
                i.complete());
            },
            (c) => {
              var u;
              ((a = !1),
                (u = r.error) === null || u === void 0 || u.call(r, c),
                i.error(c));
            },
            () => {
              var c, u;
              (a && ((c = r.unsubscribe) === null || c === void 0 || c.call(r)),
                (u = r.finalize) === null || u === void 0 || u.call(r));
            },
          ),
        );
      })
    : pe;
}
function iw(...e) {
  let t = $n(e);
  return x((n, r) => {
    let o = e.length,
      i = new Array(o),
      s = e.map(() => !1),
      a = !1;
    for (let c = 0; c < o; c++)
      V(e[c]).subscribe(
        T(
          r,
          (u) => {
            ((i[c] = u),
              !a && !s[c] && ((s[c] = !0), (a = s.every(pe)) && (s = null)));
          },
          ct,
        ),
      );
    n.subscribe(
      T(r, (c) => {
        if (a) {
          let u = [c, ...i];
          r.next(t ? t(...u) : u);
        }
      }),
    );
  });
}
var op =
    "https://angular.dev/best-practices/security#preventing-cross-site-scripting-xss",
  v = class extends Error {
    code;
    constructor(t, n) {
      (super(yo(t, n)), (this.code = t));
    }
  };
function yo(e, t) {
  return `${`NG0${Math.abs(e)}`}${t ? ": " + t : ""}`;
}
var Rs = Symbol("InputSignalNode#UNSET"),
  ip = j(D({}, Wr), {
    transformFn: void 0,
    applyValueToInputSignal(e, t) {
      tn(e, t);
    },
  });
function sp(e, t) {
  let n = Object.create(ip);
  ((n.value = e), (n.transformFn = t?.transform));
  function r() {
    if ((Ot(n), n.value === Rs)) {
      let o = null;
      throw new v(-950, o);
    }
    return n.value;
  }
  return ((r[re] = n), r);
}
function vo(e) {
  return { toString: e }.toString();
}
var $i = "__parameters__";
function sw(e) {
  return function (...n) {
    if (e) {
      let r = e(...n);
      for (let o in r) this[o] = r[o];
    }
  };
}
function ap(e, t, n) {
  return vo(() => {
    let r = sw(t);
    function o(...i) {
      if (this instanceof o) return (r.apply(this, i), this);
      let s = new o(...i);
      return ((a.annotation = s), a);
      function a(c, u, l) {
        let d = c.hasOwnProperty($i)
          ? c[$i]
          : Object.defineProperty(c, $i, { value: [] })[$i];
        for (; d.length <= l; ) d.push(null);
        return ((d[l] = d[l] || []).push(s), c);
      }
    }
    return ((o.prototype.ngMetadataName = e), (o.annotationCls = o), o);
  });
}
var ft = globalThis;
function z(e) {
  for (let t in e) if (e[t] === z) return t;
  throw Error("Could not find renamed property on target object.");
}
function aw(e, t) {
  for (let n in t) t.hasOwnProperty(n) && !e.hasOwnProperty(n) && (e[n] = t[n]);
}
function Ce(e) {
  if (typeof e == "string") return e;
  if (Array.isArray(e)) return `[${e.map(Ce).join(", ")}]`;
  if (e == null) return "" + e;
  let t = e.overriddenName || e.name;
  if (t) return `${t}`;
  let n = e.toString();
  if (n == null) return "" + n;
  let r = n.indexOf(`
`);
  return r >= 0 ? n.slice(0, r) : n;
}
function Rc(e, t) {
  return e ? (t ? `${e} ${t}` : e) : t || "";
}
var cw = z({ __forward_ref__: z });
function cp(e) {
  return (
    (e.__forward_ref__ = cp),
    (e.toString = function () {
      return Ce(this());
    }),
    e
  );
}
function ae(e) {
  return up(e) ? e() : e;
}
function up(e) {
  return (
    typeof e == "function" && e.hasOwnProperty(cw) && e.__forward_ref__ === cp
  );
}
function uw(e, t, n) {
  e != t && lw(n, e, t, "==");
}
function lw(e, t, n, r) {
  throw new Error(
    `ASSERTION ERROR: ${e}` +
      (r == null ? "" : ` [Expected=> ${n} ${r} ${t} <=Actual]`),
  );
}
function E(e) {
  return {
    token: e.token,
    providedIn: e.providedIn || null,
    factory: e.factory,
    value: void 0,
  };
}
function hr(e) {
  return { providers: e.providers || [], imports: e.imports || [] };
}
function As(e) {
  return rh(e, dp) || rh(e, fp);
}
function lp(e) {
  return As(e) !== null;
}
function rh(e, t) {
  return e.hasOwnProperty(t) ? e[t] : null;
}
function dw(e) {
  let t = e && (e[dp] || e[fp]);
  return t || null;
}
function oh(e) {
  return e && (e.hasOwnProperty(ih) || e.hasOwnProperty(fw)) ? e[ih] : null;
}
var dp = z({ ɵprov: z }),
  ih = z({ ɵinj: z }),
  fp = z({ ngInjectableDef: z }),
  fw = z({ ngInjectorDef: z }),
  I = class {
    _desc;
    ngMetadataName = "InjectionToken";
    ɵprov;
    constructor(t, n) {
      ((this._desc = t),
        (this.ɵprov = void 0),
        typeof n == "number"
          ? (this.__NG_ELEMENT_ID__ = n)
          : n !== void 0 &&
            (this.ɵprov = E({
              token: this,
              providedIn: n.providedIn || "root",
              factory: n.factory,
            })));
    }
    get multi() {
      return this;
    }
    toString() {
      return `InjectionToken ${this._desc}`;
    }
  };
function hp(e) {
  return e && !!e.ɵproviders;
}
var hw = z({ ɵcmp: z }),
  pw = z({ ɵdir: z }),
  gw = z({ ɵpipe: z }),
  mw = z({ ɵmod: z }),
  ts = z({ ɵfac: z }),
  to = z({ __NG_ELEMENT_ID__: z }),
  sh = z({ __NG_ENV_ID__: z });
function Ut(e) {
  return typeof e == "string" ? e : e == null ? "" : String(e);
}
function yw(e) {
  return typeof e == "function"
    ? e.name || e.toString()
    : typeof e == "object" && e != null && typeof e.type == "function"
      ? e.type.name || e.type.toString()
      : Ut(e);
}
function pp(e, t) {
  throw new v(-200, e);
}
function Ju(e, t) {
  throw new v(-201, !1);
}
var P = (function (e) {
    return (
      (e[(e.Default = 0)] = "Default"),
      (e[(e.Host = 1)] = "Host"),
      (e[(e.Self = 2)] = "Self"),
      (e[(e.SkipSelf = 4)] = "SkipSelf"),
      (e[(e.Optional = 8)] = "Optional"),
      e
    );
  })(P || {}),
  Ac;
function gp() {
  return Ac;
}
function Ie(e) {
  let t = Ac;
  return ((Ac = e), t);
}
function mp(e, t, n) {
  let r = As(e);
  if (r && r.providedIn == "root")
    return r.value === void 0 ? (r.value = r.factory()) : r.value;
  if (n & P.Optional) return null;
  if (t !== void 0) return t;
  Ju(e, "Injector");
}
var vw = {},
  un = vw,
  Oc = "__NG_DI_FLAG__",
  ns = class {
    injector;
    constructor(t) {
      this.injector = t;
    }
    retrieve(t, n) {
      let r = n;
      return this.injector.get(t, r.optional ? gi : un, r);
    }
  },
  rs = "ngTempTokenPath",
  Dw = "ngTokenPath",
  ww = /\n/gm,
  Ew = "\u0275",
  ah = "__source";
function Iw(e, t = P.Default) {
  if (Zr() === void 0) throw new v(-203, !1);
  if (Zr() === null) return mp(e, void 0, t);
  {
    let n = Zr(),
      r;
    return (
      n instanceof ns ? (r = n.injector) : (r = n),
      r.get(e, t & P.Optional ? null : void 0, t)
    );
  }
}
function b(e, t = P.Default) {
  return (gp() || Iw)(ae(e), t);
}
function m(e, t = P.Default) {
  return b(e, Os(t));
}
function Os(e) {
  return typeof e > "u" || typeof e == "number"
    ? e
    : 0 | (e.optional && 8) | (e.host && 1) | (e.self && 2) | (e.skipSelf && 4);
}
function kc(e) {
  let t = [];
  for (let n = 0; n < e.length; n++) {
    let r = ae(e[n]);
    if (Array.isArray(r)) {
      if (r.length === 0) throw new v(900, !1);
      let o,
        i = P.Default;
      for (let s = 0; s < r.length; s++) {
        let a = r[s],
          c = Cw(a);
        typeof c == "number" ? (c === -1 ? (o = a.token) : (i |= c)) : (o = a);
      }
      t.push(b(o, i));
    } else t.push(b(r));
  }
  return t;
}
function yp(e, t) {
  return ((e[Oc] = t), (e.prototype[Oc] = t), e);
}
function Cw(e) {
  return e[Oc];
}
function bw(e, t, n, r) {
  let o = e[rs];
  throw (
    t[ah] && o.unshift(t[ah]),
    (e.message = Mw(
      `
` + e.message,
      o,
      n,
      r,
    )),
    (e[Dw] = o),
    (e[rs] = null),
    e
  );
}
function Mw(e, t, n, r = null) {
  e =
    e &&
    e.charAt(0) ===
      `
` &&
    e.charAt(1) == Ew
      ? e.slice(2)
      : e;
  let o = Ce(t);
  if (Array.isArray(t)) o = t.map(Ce).join(" -> ");
  else if (typeof t == "object") {
    let i = [];
    for (let s in t)
      if (t.hasOwnProperty(s)) {
        let a = t[s];
        i.push(s + ":" + (typeof a == "string" ? JSON.stringify(a) : Ce(a)));
      }
    o = `{${i.join(", ")}}`;
  }
  return `${n}${r ? "(" + r + ")" : ""}[${o}]: ${e.replace(
    ww,
    `
  `,
  )}`;
}
var Xu = yp(ap("Optional"), 8);
var vp = yp(ap("SkipSelf"), 4);
function dn(e, t) {
  let n = e.hasOwnProperty(ts);
  return n ? e[ts] : null;
}
function Sw(e, t, n) {
  if (e.length !== t.length) return !1;
  for (let r = 0; r < e.length; r++) {
    let o = e[r],
      i = t[r];
    if ((n && ((o = n(o)), (i = n(i))), i !== o)) return !1;
  }
  return !0;
}
function Tw(e) {
  return e.flat(Number.POSITIVE_INFINITY);
}
function el(e, t) {
  e.forEach((n) => (Array.isArray(n) ? el(n, t) : t(n)));
}
function Dp(e, t, n) {
  t >= e.length ? e.push(n) : e.splice(t, 0, n);
}
function os(e, t) {
  return t >= e.length - 1 ? e.pop() : e.splice(t, 1)[0];
}
function _w(e, t) {
  let n = [];
  for (let r = 0; r < e; r++) n.push(t);
  return n;
}
function xw(e, t, n, r) {
  let o = e.length;
  if (o == t) e.push(n, r);
  else if (o === 1) (e.push(r, e[0]), (e[0] = n));
  else {
    for (o--, e.push(e[o - 1], e[o]); o > t; ) {
      let i = o - 2;
      ((e[o] = e[i]), o--);
    }
    ((e[t] = n), (e[t + 1] = r));
  }
}
function Do(e, t, n) {
  let r = wo(e, t);
  return (r >= 0 ? (e[r | 1] = n) : ((r = ~r), xw(e, r, t, n)), r);
}
function pc(e, t) {
  let n = wo(e, t);
  if (n >= 0) return e[n | 1];
}
function wo(e, t) {
  return Nw(e, t, 1);
}
function Nw(e, t, n) {
  let r = 0,
    o = e.length >> n;
  for (; o !== r; ) {
    let i = r + ((o - r) >> 1),
      s = e[i << n];
    if (t === s) return i << n;
    s > t ? (o = i) : (r = i + 1);
  }
  return ~(o << n);
}
var We = {},
  de = [],
  er = new I(""),
  wp = new I("", -1),
  Ep = new I(""),
  is = class {
    get(t, n = un) {
      if (n === un) {
        let r = new Error(`NullInjectorError: No provider for ${Ce(t)}!`);
        throw ((r.name = "NullInjectorError"), r);
      }
      return n;
    }
  };
function Ip(e, t) {
  let n = e[mw] || null;
  if (!n && t === !0)
    throw new Error(`Type ${Ce(e)} does not have '\u0275mod' property.`);
  return n;
}
function $t(e) {
  return e[hw] || null;
}
function Cp(e) {
  return e[pw] || null;
}
function Rw(e) {
  return e[gw] || null;
}
function pr(e) {
  return { ɵproviders: e };
}
function Aw(...e) {
  return { ɵproviders: bp(!0, e), ɵfromNgModule: !0 };
}
function bp(e, ...t) {
  let n = [],
    r = new Set(),
    o,
    i = (s) => {
      n.push(s);
    };
  return (
    el(t, (s) => {
      let a = s;
      Pc(a, i, [], r) && ((o ||= []), o.push(a));
    }),
    o !== void 0 && Mp(o, i),
    n
  );
}
function Mp(e, t) {
  for (let n = 0; n < e.length; n++) {
    let { ngModule: r, providers: o } = e[n];
    tl(o, (i) => {
      t(i, r);
    });
  }
}
function Pc(e, t, n, r) {
  if (((e = ae(e)), !e)) return !1;
  let o = null,
    i = oh(e),
    s = !i && $t(e);
  if (!i && !s) {
    let c = e.ngModule;
    if (((i = oh(c)), i)) o = c;
    else return !1;
  } else {
    if (s && !s.standalone) return !1;
    o = e;
  }
  let a = r.has(o);
  if (s) {
    if (a) return !1;
    if ((r.add(o), s.dependencies)) {
      let c =
        typeof s.dependencies == "function" ? s.dependencies() : s.dependencies;
      for (let u of c) Pc(u, t, n, r);
    }
  } else if (i) {
    if (i.imports != null && !a) {
      r.add(o);
      let u;
      try {
        el(i.imports, (l) => {
          Pc(l, t, n, r) && ((u ||= []), u.push(l));
        });
      } finally {
      }
      u !== void 0 && Mp(u, t);
    }
    if (!a) {
      let u = dn(o) || (() => new o());
      (t({ provide: o, useFactory: u, deps: de }, o),
        t({ provide: Ep, useValue: o, multi: !0 }, o),
        t({ provide: er, useValue: () => b(o), multi: !0 }, o));
    }
    let c = i.providers;
    if (c != null && !a) {
      let u = e;
      tl(c, (l) => {
        t(l, u);
      });
    }
  } else return !1;
  return o !== e && e.providers !== void 0;
}
function tl(e, t) {
  for (let n of e)
    (hp(n) && (n = n.ɵproviders), Array.isArray(n) ? tl(n, t) : t(n));
}
var Ow = z({ provide: String, useValue: z });
function Sp(e) {
  return e !== null && typeof e == "object" && Ow in e;
}
function kw(e) {
  return !!(e && e.useExisting);
}
function Pw(e) {
  return !!(e && e.useFactory);
}
function tr(e) {
  return typeof e == "function";
}
function Fw(e) {
  return !!e.useClass;
}
var ks = new I(""),
  Wi = {},
  ch = {},
  gc;
function Ps() {
  return (gc === void 0 && (gc = new is()), gc);
}
var ye = class {},
  so = class extends ye {
    parent;
    source;
    scopes;
    records = new Map();
    _ngOnDestroyHooks = new Set();
    _onDestroyHooks = [];
    get destroyed() {
      return this._destroyed;
    }
    _destroyed = !1;
    injectorDefTypes;
    constructor(t, n, r, o) {
      (super(),
        (this.parent = n),
        (this.source = r),
        (this.scopes = o),
        Lc(t, (s) => this.processProvider(s)),
        this.records.set(wp, Zn(void 0, this)),
        o.has("environment") && this.records.set(ye, Zn(void 0, this)));
      let i = this.records.get(ks);
      (i != null && typeof i.value == "string" && this.scopes.add(i.value),
        (this.injectorDefTypes = new Set(this.get(Ep, de, P.Self))));
    }
    retrieve(t, n) {
      let r = n;
      return this.get(t, r.optional ? gi : un, r);
    }
    destroy() {
      (Xr(this), (this._destroyed = !0));
      let t = A(null);
      try {
        for (let r of this._ngOnDestroyHooks) r.ngOnDestroy();
        let n = this._onDestroyHooks;
        this._onDestroyHooks = [];
        for (let r of n) r();
      } finally {
        (this.records.clear(),
          this._ngOnDestroyHooks.clear(),
          this.injectorDefTypes.clear(),
          A(t));
      }
    }
    onDestroy(t) {
      return (
        Xr(this),
        this._onDestroyHooks.push(t),
        () => this.removeOnDestroy(t)
      );
    }
    runInContext(t) {
      Xr(this);
      let n = at(this),
        r = Ie(void 0),
        o;
      try {
        return t();
      } finally {
        (at(n), Ie(r));
      }
    }
    get(t, n = un, r = P.Default) {
      if ((Xr(this), t.hasOwnProperty(sh))) return t[sh](this);
      r = Os(r);
      let o,
        i = at(this),
        s = Ie(void 0);
      try {
        if (!(r & P.SkipSelf)) {
          let c = this.records.get(t);
          if (c === void 0) {
            let u = Uw(t) && As(t);
            (u && this.injectableDefInScope(u)
              ? (c = Zn(Fc(t), Wi))
              : (c = null),
              this.records.set(t, c));
          }
          if (c != null) return this.hydrate(t, c);
        }
        let a = r & P.Self ? Ps() : this.parent;
        return ((n = r & P.Optional && n === un ? null : n), a.get(t, n));
      } catch (a) {
        if (a.name === "NullInjectorError") {
          if (((a[rs] = a[rs] || []).unshift(Ce(t)), i)) throw a;
          return bw(a, t, "R3InjectorError", this.source);
        } else throw a;
      } finally {
        (Ie(s), at(i));
      }
    }
    resolveInjectorInitializers() {
      let t = A(null),
        n = at(this),
        r = Ie(void 0),
        o;
      try {
        let i = this.get(er, de, P.Self);
        for (let s of i) s();
      } finally {
        (at(n), Ie(r), A(t));
      }
    }
    toString() {
      let t = [],
        n = this.records;
      for (let r of n.keys()) t.push(Ce(r));
      return `R3Injector[${t.join(", ")}]`;
    }
    processProvider(t) {
      t = ae(t);
      let n = tr(t) ? t : ae(t && t.provide),
        r = jw(t);
      if (!tr(t) && t.multi === !0) {
        let o = this.records.get(n);
        (o ||
          ((o = Zn(void 0, Wi, !0)),
          (o.factory = () => kc(o.multi)),
          this.records.set(n, o)),
          (n = t),
          o.multi.push(t));
      }
      this.records.set(n, r);
    }
    hydrate(t, n) {
      let r = A(null);
      try {
        return (
          n.value === ch
            ? pp(Ce(t))
            : n.value === Wi && ((n.value = ch), (n.value = n.factory())),
          typeof n.value == "object" &&
            n.value &&
            Bw(n.value) &&
            this._ngOnDestroyHooks.add(n.value),
          n.value
        );
      } finally {
        A(r);
      }
    }
    injectableDefInScope(t) {
      if (!t.providedIn) return !1;
      let n = ae(t.providedIn);
      return typeof n == "string"
        ? n === "any" || this.scopes.has(n)
        : this.injectorDefTypes.has(n);
    }
    removeOnDestroy(t) {
      let n = this._onDestroyHooks.indexOf(t);
      n !== -1 && this._onDestroyHooks.splice(n, 1);
    }
  };
function Fc(e) {
  let t = As(e),
    n = t !== null ? t.factory : dn(e);
  if (n !== null) return n;
  if (e instanceof I) throw new v(204, !1);
  if (e instanceof Function) return Lw(e);
  throw new v(204, !1);
}
function Lw(e) {
  if (e.length > 0) throw new v(204, !1);
  let n = dw(e);
  return n !== null ? () => n.factory(e) : () => new e();
}
function jw(e) {
  if (Sp(e)) return Zn(void 0, e.useValue);
  {
    let t = Tp(e);
    return Zn(t, Wi);
  }
}
function Tp(e, t, n) {
  let r;
  if (tr(e)) {
    let o = ae(e);
    return dn(o) || Fc(o);
  } else if (Sp(e)) r = () => ae(e.useValue);
  else if (Pw(e)) r = () => e.useFactory(...kc(e.deps || []));
  else if (kw(e)) r = () => b(ae(e.useExisting));
  else {
    let o = ae(e && (e.useClass || e.provide));
    if (Vw(e)) r = () => new o(...kc(e.deps));
    else return dn(o) || Fc(o);
  }
  return r;
}
function Xr(e) {
  if (e.destroyed) throw new v(205, !1);
}
function Zn(e, t, n = !1) {
  return { factory: e, value: t, multi: n ? [] : void 0 };
}
function Vw(e) {
  return !!e.deps;
}
function Bw(e) {
  return (
    e !== null && typeof e == "object" && typeof e.ngOnDestroy == "function"
  );
}
function Uw(e) {
  return typeof e == "function" || (typeof e == "object" && e instanceof I);
}
function Lc(e, t) {
  for (let n of e)
    Array.isArray(n) ? Lc(n, t) : n && hp(n) ? Lc(n.ɵproviders, t) : t(n);
}
function Me(e, t) {
  let n;
  e instanceof so ? (Xr(e), (n = e)) : (n = new ns(e));
  let r,
    o = at(n),
    i = Ie(void 0);
  try {
    return t();
  } finally {
    (at(o), Ie(i));
  }
}
function _p() {
  return gp() !== void 0 || Zr() != null;
}
function gr(e) {
  if (!_p()) throw new v(-203, !1);
}
function $w(e) {
  return typeof e == "function";
}
var Dt = 0,
  N = 1,
  _ = 2,
  he = 3,
  Be = 4,
  De = 5,
  nr = 6,
  ss = 7,
  ne = 8,
  rr = 9,
  ht = 10,
  U = 11,
  ao = 12,
  uh = 13,
  mr = 14,
  be = 15,
  fn = 16,
  Yn = 17,
  pt = 18,
  Fs = 19,
  xp = 20,
  Vt = 21,
  mc = 22,
  hn = 23,
  ke = 24,
  Jn = 25,
  Y = 26,
  Np = 1;
var pn = 7,
  as = 8,
  or = 9,
  fe = 10;
function Bt(e) {
  return Array.isArray(e) && typeof e[Np] == "object";
}
function wt(e) {
  return Array.isArray(e) && e[Np] === !0;
}
function nl(e) {
  return (e.flags & 4) !== 0;
}
function yr(e) {
  return e.componentOffset > -1;
}
function Ls(e) {
  return (e.flags & 1) === 1;
}
function Ze(e) {
  return !!e.template;
}
function cs(e) {
  return (e[_] & 512) !== 0;
}
function Eo(e) {
  return (e[_] & 256) === 256;
}
var jc = class {
  previousValue;
  currentValue;
  firstChange;
  constructor(t, n, r) {
    ((this.previousValue = t), (this.currentValue = n), (this.firstChange = r));
  }
  isFirstChange() {
    return this.firstChange;
  }
};
function Rp(e, t, n, r) {
  t !== null ? t.applyValueToInputSignal(t, r) : (e[n] = r);
}
var vr = (() => {
  let e = () => Ap;
  return ((e.ngInherit = !0), e);
})();
function Ap(e) {
  return (e.type.prototype.ngOnChanges && (e.setInput = zw), Hw);
}
function Hw() {
  let e = kp(this),
    t = e?.current;
  if (t) {
    let n = e.previous;
    if (n === We) e.previous = t;
    else for (let r in t) n[r] = t[r];
    ((e.current = null), this.ngOnChanges(t));
  }
}
function zw(e, t, n, r, o) {
  let i = this.declaredInputs[r],
    s = kp(e) || qw(e, { previous: We, current: null }),
    a = s.current || (s.current = {}),
    c = s.previous,
    u = c[i];
  ((a[i] = new jc(u && u.currentValue, n, c === We)), Rp(e, t, o, n));
}
var Op = "__ngSimpleChanges__";
function kp(e) {
  return e[Op] || null;
}
function qw(e, t) {
  return (e[Op] = t);
}
var lh = null;
var $ = function (e, t = null, n) {
    lh?.(e, t, n);
  },
  Pp = "svg",
  Gw = "math";
function Pe(e) {
  for (; Array.isArray(e); ) e = e[Dt];
  return e;
}
function js(e, t) {
  return Pe(t[e]);
}
function Xe(e, t) {
  return Pe(t[e.index]);
}
function rl(e, t) {
  return e.data[t];
}
function Io(e, t) {
  return e[t];
}
function Ye(e, t) {
  let n = t[e];
  return Bt(n) ? n : n[Dt];
}
function Ww(e) {
  return (e[_] & 4) === 4;
}
function ol(e) {
  return (e[_] & 128) === 128;
}
function Zw(e) {
  return wt(e[he]);
}
function gt(e, t) {
  return t == null ? null : e[t];
}
function Fp(e) {
  e[Yn] = 0;
}
function Lp(e) {
  e[_] & 1024 || ((e[_] |= 1024), ol(e) && Dr(e));
}
function Yw(e, t) {
  for (; e > 0; ) ((t = t[mr]), e--);
  return t;
}
function Vs(e) {
  return !!(e[_] & 9216 || e[ke]?.dirty);
}
function Vc(e) {
  (e[ht].changeDetectionScheduler?.notify(8),
    e[_] & 64 && (e[_] |= 1024),
    Vs(e) && Dr(e));
}
function Dr(e) {
  e[ht].changeDetectionScheduler?.notify(0);
  let t = gn(e);
  for (; t !== null && !(t[_] & 8192 || ((t[_] |= 8192), !ol(t))); ) t = gn(t);
}
function jp(e, t) {
  if (Eo(e)) throw new v(911, !1);
  (e[Vt] === null && (e[Vt] = []), e[Vt].push(t));
}
function Qw(e, t) {
  if (e[Vt] === null) return;
  let n = e[Vt].indexOf(t);
  n !== -1 && e[Vt].splice(n, 1);
}
function gn(e) {
  let t = e[he];
  return wt(t) ? t[he] : t;
}
function Vp(e) {
  return (e[ss] ??= []);
}
function Bp(e) {
  return (e.cleanup ??= []);
}
function Kw(e, t, n, r) {
  let o = Vp(t);
  (o.push(n), e.firstCreatePass && Bp(e).push(r, o.length - 1));
}
var R = { lFrame: Zp(null), bindingsEnabled: !0, skipHydrationRootTNode: null };
var Bc = !1;
function Jw() {
  return R.lFrame.elementDepthCount;
}
function Xw() {
  R.lFrame.elementDepthCount++;
}
function eE() {
  R.lFrame.elementDepthCount--;
}
function il() {
  return R.bindingsEnabled;
}
function Up() {
  return R.skipHydrationRootTNode !== null;
}
function tE(e) {
  return R.skipHydrationRootTNode === e;
}
function nE() {
  R.skipHydrationRootTNode = null;
}
function C() {
  return R.lFrame.lView;
}
function Q() {
  return R.lFrame.tView;
}
function o1(e) {
  return ((R.lFrame.contextLView = e), e[ne]);
}
function i1(e) {
  return ((R.lFrame.contextLView = null), e);
}
function we() {
  let e = $p();
  for (; e !== null && e.type === 64; ) e = e.parent;
  return e;
}
function $p() {
  return R.lFrame.currentTNode;
}
function co() {
  let e = R.lFrame,
    t = e.currentTNode;
  return e.isParent ? t : t.parent;
}
function Qe(e, t) {
  let n = R.lFrame;
  ((n.currentTNode = e), (n.isParent = t));
}
function sl() {
  return R.lFrame.isParent;
}
function al() {
  R.lFrame.isParent = !1;
}
function rE() {
  return R.lFrame.contextLView;
}
function Hp() {
  return Bc;
}
function us(e) {
  let t = Bc;
  return ((Bc = e), t);
}
function et() {
  let e = R.lFrame,
    t = e.bindingRootIndex;
  return (t === -1 && (t = e.bindingRootIndex = e.tView.bindingStartIndex), t);
}
function zp() {
  return R.lFrame.bindingIndex;
}
function oE(e) {
  return (R.lFrame.bindingIndex = e);
}
function Cn() {
  return R.lFrame.bindingIndex++;
}
function cl(e) {
  let t = R.lFrame,
    n = t.bindingIndex;
  return ((t.bindingIndex = t.bindingIndex + e), n);
}
function iE() {
  return R.lFrame.inI18n;
}
function qp(e) {
  R.lFrame.inI18n = e;
}
function sE(e, t) {
  let n = R.lFrame;
  ((n.bindingIndex = n.bindingRootIndex = e), Uc(t));
}
function aE() {
  return R.lFrame.currentDirectiveIndex;
}
function Uc(e) {
  R.lFrame.currentDirectiveIndex = e;
}
function cE(e) {
  let t = R.lFrame.currentDirectiveIndex;
  return t === -1 ? null : e[t];
}
function ul() {
  return R.lFrame.currentQueryIndex;
}
function Bs(e) {
  R.lFrame.currentQueryIndex = e;
}
function uE(e) {
  let t = e[N];
  return t.type === 2 ? t.declTNode : t.type === 1 ? e[De] : null;
}
function Gp(e, t, n) {
  if (n & P.SkipSelf) {
    let o = t,
      i = e;
    for (; (o = o.parent), o === null && !(n & P.Host); )
      if (((o = uE(i)), o === null || ((i = i[mr]), o.type & 10))) break;
    if (o === null) return !1;
    ((t = o), (e = i));
  }
  let r = (R.lFrame = Wp());
  return ((r.currentTNode = t), (r.lView = e), !0);
}
function ll(e) {
  let t = Wp(),
    n = e[N];
  ((R.lFrame = t),
    (t.currentTNode = n.firstChild),
    (t.lView = e),
    (t.tView = n),
    (t.contextLView = e),
    (t.bindingIndex = n.bindingStartIndex),
    (t.inI18n = !1));
}
function Wp() {
  let e = R.lFrame,
    t = e === null ? null : e.child;
  return t === null ? Zp(e) : t;
}
function Zp(e) {
  let t = {
    currentTNode: null,
    isParent: !0,
    lView: null,
    tView: null,
    selectedIndex: -1,
    contextLView: null,
    elementDepthCount: 0,
    currentNamespace: null,
    currentDirectiveIndex: -1,
    bindingRootIndex: -1,
    bindingIndex: -1,
    currentQueryIndex: 0,
    parent: e,
    child: null,
    inI18n: !1,
  };
  return (e !== null && (e.child = t), t);
}
function Yp() {
  let e = R.lFrame;
  return ((R.lFrame = e.parent), (e.currentTNode = null), (e.lView = null), e);
}
var Qp = Yp;
function dl() {
  let e = Yp();
  ((e.isParent = !0),
    (e.tView = null),
    (e.selectedIndex = -1),
    (e.contextLView = null),
    (e.elementDepthCount = 0),
    (e.currentDirectiveIndex = -1),
    (e.currentNamespace = null),
    (e.bindingRootIndex = -1),
    (e.bindingIndex = -1),
    (e.currentQueryIndex = 0));
}
function lE(e) {
  return (R.lFrame.contextLView = Yw(e, R.lFrame.contextLView))[ne];
}
function Et() {
  return R.lFrame.selectedIndex;
}
function mn(e) {
  R.lFrame.selectedIndex = e;
}
function Us() {
  let e = R.lFrame;
  return rl(e.tView, e.selectedIndex);
}
function s1() {
  R.lFrame.currentNamespace = Pp;
}
function a1() {
  dE();
}
function dE() {
  R.lFrame.currentNamespace = null;
}
function fE() {
  return R.lFrame.currentNamespace;
}
var Kp = !0;
function Co() {
  return Kp;
}
function bo(e) {
  Kp = e;
}
function hE(e, t, n) {
  let { ngOnChanges: r, ngOnInit: o, ngDoCheck: i } = t.type.prototype;
  if (r) {
    let s = Ap(t);
    ((n.preOrderHooks ??= []).push(e, s),
      (n.preOrderCheckHooks ??= []).push(e, s));
  }
  (o && (n.preOrderHooks ??= []).push(0 - e, o),
    i &&
      ((n.preOrderHooks ??= []).push(e, i),
      (n.preOrderCheckHooks ??= []).push(e, i)));
}
function fl(e, t) {
  for (let n = t.directiveStart, r = t.directiveEnd; n < r; n++) {
    let i = e.data[n].type.prototype,
      {
        ngAfterContentInit: s,
        ngAfterContentChecked: a,
        ngAfterViewInit: c,
        ngAfterViewChecked: u,
        ngOnDestroy: l,
      } = i;
    (s && (e.contentHooks ??= []).push(-n, s),
      a &&
        ((e.contentHooks ??= []).push(n, a),
        (e.contentCheckHooks ??= []).push(n, a)),
      c && (e.viewHooks ??= []).push(-n, c),
      u &&
        ((e.viewHooks ??= []).push(n, u), (e.viewCheckHooks ??= []).push(n, u)),
      l != null && (e.destroyHooks ??= []).push(n, l));
  }
}
function Zi(e, t, n) {
  Jp(e, t, 3, n);
}
function Yi(e, t, n, r) {
  (e[_] & 3) === n && Jp(e, t, n, r);
}
function yc(e, t) {
  let n = e[_];
  (n & 3) === t && ((n &= 16383), (n += 1), (e[_] = n));
}
function Jp(e, t, n, r) {
  let o = r !== void 0 ? e[Yn] & 65535 : 0,
    i = r ?? -1,
    s = t.length - 1,
    a = 0;
  for (let c = o; c < s; c++)
    if (typeof t[c + 1] == "number") {
      if (((a = t[c]), r != null && a >= r)) break;
    } else
      (t[c] < 0 && (e[Yn] += 65536),
        (a < i || i == -1) &&
          (pE(e, n, t, c), (e[Yn] = (e[Yn] & 4294901760) + c + 2)),
        c++);
}
function dh(e, t) {
  $(4, e, t);
  let n = A(null);
  try {
    t.call(e);
  } finally {
    (A(n), $(5, e, t));
  }
}
function pE(e, t, n, r) {
  let o = n[r] < 0,
    i = n[r + 1],
    s = o ? -n[r] : n[r],
    a = e[s];
  o
    ? e[_] >> 14 < e[Yn] >> 16 &&
      (e[_] & 3) === t &&
      ((e[_] += 16384), dh(a, i))
    : dh(a, i);
}
var Xn = -1,
  yn = class {
    factory;
    injectImpl;
    resolving = !1;
    canSeeViewProviders;
    multi;
    componentProviders;
    index;
    providerFactory;
    constructor(t, n, r) {
      ((this.factory = t),
        (this.canSeeViewProviders = n),
        (this.injectImpl = r));
    }
  };
function gE(e) {
  return (e.flags & 8) !== 0;
}
function mE(e) {
  return (e.flags & 16) !== 0;
}
function yE(e, t, n) {
  let r = 0;
  for (; r < n.length; ) {
    let o = n[r];
    if (typeof o == "number") {
      if (o !== 0) break;
      r++;
      let i = n[r++],
        s = n[r++],
        a = n[r++];
      e.setAttribute(t, s, a, i);
    } else {
      let i = o,
        s = n[++r];
      (vE(i) ? e.setProperty(t, i, s) : e.setAttribute(t, i, s), r++);
    }
  }
  return r;
}
function Xp(e) {
  return e === 3 || e === 4 || e === 6;
}
function vE(e) {
  return e.charCodeAt(0) === 64;
}
function ir(e, t) {
  if (!(t === null || t.length === 0))
    if (e === null || e.length === 0) e = t.slice();
    else {
      let n = -1;
      for (let r = 0; r < t.length; r++) {
        let o = t[r];
        typeof o == "number"
          ? (n = o)
          : n === 0 ||
            (n === -1 || n === 2
              ? fh(e, n, o, null, t[++r])
              : fh(e, n, o, null, null));
      }
    }
  return e;
}
function fh(e, t, n, r, o) {
  let i = 0,
    s = e.length;
  if (t === -1) s = -1;
  else
    for (; i < e.length; ) {
      let a = e[i++];
      if (typeof a == "number") {
        if (a === t) {
          s = -1;
          break;
        } else if (a > t) {
          s = i - 1;
          break;
        }
      }
    }
  for (; i < e.length; ) {
    let a = e[i];
    if (typeof a == "number") break;
    if (a === n) {
      o !== null && (e[i + 1] = o);
      return;
    }
    (i++, o !== null && i++);
  }
  (s !== -1 && (e.splice(s, 0, t), (i = s + 1)),
    e.splice(i++, 0, n),
    o !== null && e.splice(i++, 0, o));
}
var vc = {},
  $c = class {
    injector;
    parentInjector;
    constructor(t, n) {
      ((this.injector = t), (this.parentInjector = n));
    }
    get(t, n, r) {
      r = Os(r);
      let o = this.injector.get(t, vc, r);
      return o !== vc || n === vc ? o : this.parentInjector.get(t, n, r);
    }
  };
function eg(e) {
  return e !== Xn;
}
function ls(e) {
  return e & 32767;
}
function DE(e) {
  return e >> 16;
}
function ds(e, t) {
  let n = DE(e),
    r = t;
  for (; n > 0; ) ((r = r[mr]), n--);
  return r;
}
var Hc = !0;
function fs(e) {
  let t = Hc;
  return ((Hc = e), t);
}
var wE = 256,
  tg = wE - 1,
  ng = 5,
  EE = 0,
  Ge = {};
function IE(e, t, n) {
  let r;
  (typeof n == "string"
    ? (r = n.charCodeAt(0) || 0)
    : n.hasOwnProperty(to) && (r = n[to]),
    r == null && (r = n[to] = EE++));
  let o = r & tg,
    i = 1 << o;
  t.data[e + (o >> ng)] |= i;
}
function hs(e, t) {
  let n = rg(e, t);
  if (n !== -1) return n;
  let r = t[N];
  r.firstCreatePass &&
    ((e.injectorIndex = t.length),
    Dc(r.data, e),
    Dc(t, null),
    Dc(r.blueprint, null));
  let o = hl(e, t),
    i = e.injectorIndex;
  if (eg(o)) {
    let s = ls(o),
      a = ds(o, t),
      c = a[N].data;
    for (let u = 0; u < 8; u++) t[i + u] = a[s + u] | c[s + u];
  }
  return ((t[i + 8] = o), i);
}
function Dc(e, t) {
  e.push(0, 0, 0, 0, 0, 0, 0, 0, t);
}
function rg(e, t) {
  return e.injectorIndex === -1 ||
    (e.parent && e.parent.injectorIndex === e.injectorIndex) ||
    t[e.injectorIndex + 8] === null
    ? -1
    : e.injectorIndex;
}
function hl(e, t) {
  if (e.parent && e.parent.injectorIndex !== -1) return e.parent.injectorIndex;
  let n = 0,
    r = null,
    o = t;
  for (; o !== null; ) {
    if (((r = cg(o)), r === null)) return Xn;
    if ((n++, (o = o[mr]), r.injectorIndex !== -1))
      return r.injectorIndex | (n << 16);
  }
  return Xn;
}
function zc(e, t, n) {
  IE(e, t, n);
}
function CE(e, t) {
  if (t === "class") return e.classes;
  if (t === "style") return e.styles;
  let n = e.attrs;
  if (n) {
    let r = n.length,
      o = 0;
    for (; o < r; ) {
      let i = n[o];
      if (Xp(i)) break;
      if (i === 0) o = o + 2;
      else if (typeof i == "number")
        for (o++; o < r && typeof n[o] == "string"; ) o++;
      else {
        if (i === t) return n[o + 1];
        o = o + 2;
      }
    }
  }
  return null;
}
function og(e, t, n) {
  if (n & P.Optional || e !== void 0) return e;
  Ju(t, "NodeInjector");
}
function ig(e, t, n, r) {
  if (
    (n & P.Optional && r === void 0 && (r = null),
    (n & (P.Self | P.Host)) === 0)
  ) {
    let o = e[rr],
      i = Ie(void 0);
    try {
      return o ? o.get(t, r, n & P.Optional) : mp(t, r, n & P.Optional);
    } finally {
      Ie(i);
    }
  }
  return og(r, t, n);
}
function sg(e, t, n, r = P.Default, o) {
  if (e !== null) {
    if (t[_] & 2048 && !(r & P.Self)) {
      let s = TE(e, t, n, r, Ge);
      if (s !== Ge) return s;
    }
    let i = ag(e, t, n, r, Ge);
    if (i !== Ge) return i;
  }
  return ig(t, n, r, o);
}
function ag(e, t, n, r, o) {
  let i = ME(n);
  if (typeof i == "function") {
    if (!Gp(t, e, r)) return r & P.Host ? og(o, n, r) : ig(t, n, r, o);
    try {
      let s;
      if (((s = i(r)), s == null && !(r & P.Optional))) Ju(n);
      else return s;
    } finally {
      Qp();
    }
  } else if (typeof i == "number") {
    let s = null,
      a = rg(e, t),
      c = Xn,
      u = r & P.Host ? t[be][De] : null;
    for (
      (a === -1 || r & P.SkipSelf) &&
      ((c = a === -1 ? hl(e, t) : t[a + 8]),
      c === Xn || !ph(r, !1)
        ? (a = -1)
        : ((s = t[N]), (a = ls(c)), (t = ds(c, t))));
      a !== -1;
    ) {
      let l = t[N];
      if (hh(i, a, l.data)) {
        let d = bE(a, t, n, s, r, u);
        if (d !== Ge) return d;
      }
      ((c = t[a + 8]),
        c !== Xn && ph(r, t[N].data[a + 8] === u) && hh(i, a, t)
          ? ((s = l), (a = ls(c)), (t = ds(c, t)))
          : (a = -1));
    }
  }
  return o;
}
function bE(e, t, n, r, o, i) {
  let s = t[N],
    a = s.data[e + 8],
    c = r == null ? yr(a) && Hc : r != s && (a.type & 3) !== 0,
    u = o & P.Host && i === a,
    l = Qi(a, s, n, c, u);
  return l !== null ? uo(t, s, l, a) : Ge;
}
function Qi(e, t, n, r, o) {
  let i = e.providerIndexes,
    s = t.data,
    a = i & 1048575,
    c = e.directiveStart,
    u = e.directiveEnd,
    l = i >> 20,
    d = r ? a : a + l,
    h = o ? a + l : u;
  for (let f = d; f < h; f++) {
    let p = s[f];
    if ((f < c && n === p) || (f >= c && p.type === n)) return f;
  }
  if (o) {
    let f = s[c];
    if (f && Ze(f) && f.type === n) return c;
  }
  return null;
}
function uo(e, t, n, r) {
  let o = e[n],
    i = t.data;
  if (o instanceof yn) {
    let s = o;
    s.resolving && pp(yw(i[n]));
    let a = fs(s.canSeeViewProviders);
    s.resolving = !0;
    let c,
      u = s.injectImpl ? Ie(s.injectImpl) : null,
      l = Gp(e, r, P.Default);
    try {
      ((o = e[n] = s.factory(void 0, i, e, r)),
        t.firstCreatePass && n >= r.directiveStart && hE(n, i[n], t));
    } finally {
      (u !== null && Ie(u), fs(a), (s.resolving = !1), Qp());
    }
  }
  return o;
}
function ME(e) {
  if (typeof e == "string") return e.charCodeAt(0) || 0;
  let t = e.hasOwnProperty(to) ? e[to] : void 0;
  return typeof t == "number" ? (t >= 0 ? t & tg : SE) : t;
}
function hh(e, t, n) {
  let r = 1 << e;
  return !!(n[t + (e >> ng)] & r);
}
function ph(e, t) {
  return !(e & P.Self) && !(e & P.Host && t);
}
var ln = class {
  _tNode;
  _lView;
  constructor(t, n) {
    ((this._tNode = t), (this._lView = n));
  }
  get(t, n, r) {
    return sg(this._tNode, this._lView, t, Os(r), n);
  }
};
function SE() {
  return new ln(we(), C());
}
function pl(e) {
  return vo(() => {
    let t = e.prototype.constructor,
      n = t[ts] || qc(t),
      r = Object.prototype,
      o = Object.getPrototypeOf(e.prototype).constructor;
    for (; o && o !== r; ) {
      let i = o[ts] || qc(o);
      if (i && i !== n) return i;
      o = Object.getPrototypeOf(o);
    }
    return (i) => new i();
  });
}
function qc(e) {
  return up(e)
    ? () => {
        let t = qc(ae(e));
        return t && t();
      }
    : dn(e);
}
function TE(e, t, n, r, o) {
  let i = e,
    s = t;
  for (; i !== null && s !== null && s[_] & 2048 && !cs(s); ) {
    let a = ag(i, s, n, r | P.Self, Ge);
    if (a !== Ge) return a;
    let c = i.parent;
    if (!c) {
      let u = s[xp];
      if (u) {
        let l = u.get(n, Ge, r);
        if (l !== Ge) return l;
      }
      ((c = cg(s)), (s = s[mr]));
    }
    i = c;
  }
  return o;
}
function cg(e) {
  let t = e[N],
    n = t.type;
  return n === 2 ? t.declTNode : n === 1 ? e[De] : null;
}
function gl(e) {
  return CE(we(), e);
}
function gh(e, t = null, n = null, r) {
  let o = ug(e, t, n, r);
  return (o.resolveInjectorInitializers(), o);
}
function ug(e, t = null, n = null, r, o = new Set()) {
  let i = [n || de, Aw(e)];
  return (
    (r = r || (typeof e == "object" ? void 0 : Ce(e))),
    new so(i, t || Ps(), r || null, o)
  );
}
var ce = class e {
  static THROW_IF_NOT_FOUND = un;
  static NULL = new is();
  static create(t, n) {
    if (Array.isArray(t)) return gh({ name: "" }, n, t, "");
    {
      let r = t.name ?? "";
      return gh({ name: r }, t.parent, t.providers, r);
    }
  }
  static ɵprov = E({ token: e, providedIn: "any", factory: () => b(wp) });
  static __NG_ELEMENT_ID__ = -1;
};
var _E = new I("");
_E.__NG_ELEMENT_ID__ = (e) => {
  let t = we();
  if (t === null) throw new v(204, !1);
  if (t.type & 2) return t.value;
  if (e & P.Optional) return null;
  throw new v(204, !1);
};
var lg = !1,
  It = (() => {
    class e {
      static __NG_ELEMENT_ID__ = xE;
      static __NG_ENV_ID__ = (n) => n;
    }
    return e;
  })(),
  ps = class extends It {
    _lView;
    constructor(t) {
      (super(), (this._lView = t));
    }
    onDestroy(t) {
      return (jp(this._lView, t), () => Qw(this._lView, t));
    }
  };
function xE() {
  return new ps(C());
}
var mt = class {},
  $s = new I("", { providedIn: "root", factory: () => !1 });
var dg = new I(""),
  fg = new I(""),
  Ct = (() => {
    class e {
      taskId = 0;
      pendingTasks = new Set();
      get _hasPendingTasks() {
        return this.hasPendingTasks.value;
      }
      hasPendingTasks = new se(!1);
      add() {
        this._hasPendingTasks || this.hasPendingTasks.next(!0);
        let n = this.taskId++;
        return (this.pendingTasks.add(n), n);
      }
      has(n) {
        return this.pendingTasks.has(n);
      }
      remove(n) {
        (this.pendingTasks.delete(n),
          this.pendingTasks.size === 0 &&
            this._hasPendingTasks &&
            this.hasPendingTasks.next(!1));
      }
      ngOnDestroy() {
        (this.pendingTasks.clear(),
          this._hasPendingTasks && this.hasPendingTasks.next(!1));
      }
      static ɵprov = E({
        token: e,
        providedIn: "root",
        factory: () => new e(),
      });
    }
    return e;
  })(),
  NE = (() => {
    class e {
      internalPendingTasks = m(Ct);
      scheduler = m(mt);
      add() {
        let n = this.internalPendingTasks.add();
        return () => {
          this.internalPendingTasks.has(n) &&
            (this.scheduler.notify(11), this.internalPendingTasks.remove(n));
        };
      }
      run(n) {
        return Rn(this, null, function* () {
          let r = this.add();
          try {
            return yield n();
          } finally {
            r();
          }
        });
      }
      static ɵprov = E({
        token: e,
        providedIn: "root",
        factory: () => new e(),
      });
    }
    return e;
  })(),
  Gc = class extends X {
    __isAsync;
    destroyRef = void 0;
    pendingTasks = void 0;
    constructor(t = !1) {
      (super(),
        (this.__isAsync = t),
        _p() &&
          ((this.destroyRef = m(It, { optional: !0 }) ?? void 0),
          (this.pendingTasks = m(Ct, { optional: !0 }) ?? void 0)));
    }
    emit(t) {
      let n = A(null);
      try {
        super.next(t);
      } finally {
        A(n);
      }
    }
    subscribe(t, n, r) {
      let o = t,
        i = n || (() => null),
        s = r;
      if (t && typeof t == "object") {
        let c = t;
        ((o = c.next?.bind(c)),
          (i = c.error?.bind(c)),
          (s = c.complete?.bind(c)));
      }
      this.__isAsync &&
        ((i = this.wrapInTimeout(i)),
        o && (o = this.wrapInTimeout(o)),
        s && (s = this.wrapInTimeout(s)));
      let a = super.subscribe({ next: o, error: i, complete: s });
      return (t instanceof K && t.add(a), a);
    }
    wrapInTimeout(t) {
      return (n) => {
        let r = this.pendingTasks?.add();
        setTimeout(() => {
          (t(n), r !== void 0 && this.pendingTasks?.remove(r));
        });
      };
    }
  },
  _e = Gc;
function lo(...e) {}
function hg(e) {
  let t, n;
  function r() {
    e = lo;
    try {
      (n !== void 0 &&
        typeof cancelAnimationFrame == "function" &&
        cancelAnimationFrame(n),
        t !== void 0 && clearTimeout(t));
    } catch {}
  }
  return (
    (t = setTimeout(() => {
      (e(), r());
    })),
    typeof requestAnimationFrame == "function" &&
      (n = requestAnimationFrame(() => {
        (e(), r());
      })),
    () => r()
  );
}
function mh(e) {
  return (
    queueMicrotask(() => e()),
    () => {
      e = lo;
    }
  );
}
var ml = "isAngularZone",
  gs = ml + "_ID",
  RE = 0,
  J = class e {
    hasPendingMacrotasks = !1;
    hasPendingMicrotasks = !1;
    isStable = !0;
    onUnstable = new _e(!1);
    onMicrotaskEmpty = new _e(!1);
    onStable = new _e(!1);
    onError = new _e(!1);
    constructor(t) {
      let {
        enableLongStackTrace: n = !1,
        shouldCoalesceEventChangeDetection: r = !1,
        shouldCoalesceRunChangeDetection: o = !1,
        scheduleInRootZone: i = lg,
      } = t;
      if (typeof Zone > "u") throw new v(908, !1);
      Zone.assertZonePatched();
      let s = this;
      ((s._nesting = 0),
        (s._outer = s._inner = Zone.current),
        Zone.TaskTrackingZoneSpec &&
          (s._inner = s._inner.fork(new Zone.TaskTrackingZoneSpec())),
        n &&
          Zone.longStackTraceZoneSpec &&
          (s._inner = s._inner.fork(Zone.longStackTraceZoneSpec)),
        (s.shouldCoalesceEventChangeDetection = !o && r),
        (s.shouldCoalesceRunChangeDetection = o),
        (s.callbackScheduled = !1),
        (s.scheduleInRootZone = i),
        kE(s));
    }
    static isInAngularZone() {
      return typeof Zone < "u" && Zone.current.get(ml) === !0;
    }
    static assertInAngularZone() {
      if (!e.isInAngularZone()) throw new v(909, !1);
    }
    static assertNotInAngularZone() {
      if (e.isInAngularZone()) throw new v(909, !1);
    }
    run(t, n, r) {
      return this._inner.run(t, n, r);
    }
    runTask(t, n, r, o) {
      let i = this._inner,
        s = i.scheduleEventTask("NgZoneEvent: " + o, t, AE, lo, lo);
      try {
        return i.runTask(s, n, r);
      } finally {
        i.cancelTask(s);
      }
    }
    runGuarded(t, n, r) {
      return this._inner.runGuarded(t, n, r);
    }
    runOutsideAngular(t) {
      return this._outer.run(t);
    }
  },
  AE = {};
function yl(e) {
  if (e._nesting == 0 && !e.hasPendingMicrotasks && !e.isStable)
    try {
      (e._nesting++, e.onMicrotaskEmpty.emit(null));
    } finally {
      if ((e._nesting--, !e.hasPendingMicrotasks))
        try {
          e.runOutsideAngular(() => e.onStable.emit(null));
        } finally {
          e.isStable = !0;
        }
    }
}
function OE(e) {
  if (e.isCheckStableRunning || e.callbackScheduled) return;
  e.callbackScheduled = !0;
  function t() {
    hg(() => {
      ((e.callbackScheduled = !1),
        Wc(e),
        (e.isCheckStableRunning = !0),
        yl(e),
        (e.isCheckStableRunning = !1));
    });
  }
  (e.scheduleInRootZone
    ? Zone.root.run(() => {
        t();
      })
    : e._outer.run(() => {
        t();
      }),
    Wc(e));
}
function kE(e) {
  let t = () => {
      OE(e);
    },
    n = RE++;
  e._inner = e._inner.fork({
    name: "angular",
    properties: { [ml]: !0, [gs]: n, [gs + n]: !0 },
    onInvokeTask: (r, o, i, s, a, c) => {
      if (PE(c)) return r.invokeTask(i, s, a, c);
      try {
        return (yh(e), r.invokeTask(i, s, a, c));
      } finally {
        (((e.shouldCoalesceEventChangeDetection && s.type === "eventTask") ||
          e.shouldCoalesceRunChangeDetection) &&
          t(),
          vh(e));
      }
    },
    onInvoke: (r, o, i, s, a, c, u) => {
      try {
        return (yh(e), r.invoke(i, s, a, c, u));
      } finally {
        (e.shouldCoalesceRunChangeDetection &&
          !e.callbackScheduled &&
          !FE(c) &&
          t(),
          vh(e));
      }
    },
    onHasTask: (r, o, i, s) => {
      (r.hasTask(i, s),
        o === i &&
          (s.change == "microTask"
            ? ((e._hasPendingMicrotasks = s.microTask), Wc(e), yl(e))
            : s.change == "macroTask" &&
              (e.hasPendingMacrotasks = s.macroTask)));
    },
    onHandleError: (r, o, i, s) => (
      r.handleError(i, s),
      e.runOutsideAngular(() => e.onError.emit(s)),
      !1
    ),
  });
}
function Wc(e) {
  e._hasPendingMicrotasks ||
  ((e.shouldCoalesceEventChangeDetection ||
    e.shouldCoalesceRunChangeDetection) &&
    e.callbackScheduled === !0)
    ? (e.hasPendingMicrotasks = !0)
    : (e.hasPendingMicrotasks = !1);
}
function yh(e) {
  (e._nesting++, e.isStable && ((e.isStable = !1), e.onUnstable.emit(null)));
}
function vh(e) {
  (e._nesting--, yl(e));
}
var Zc = class {
  hasPendingMicrotasks = !1;
  hasPendingMacrotasks = !1;
  isStable = !0;
  onUnstable = new _e();
  onMicrotaskEmpty = new _e();
  onStable = new _e();
  onError = new _e();
  run(t, n, r) {
    return t.apply(n, r);
  }
  runGuarded(t, n, r) {
    return t.apply(n, r);
  }
  runOutsideAngular(t) {
    return t();
  }
  runTask(t, n, r, o) {
    return t.apply(n, r);
  }
};
function PE(e) {
  return pg(e, "__ignore_ng_zone__");
}
function FE(e) {
  return pg(e, "__scheduler_tick__");
}
function pg(e, t) {
  return !Array.isArray(e) || e.length !== 1 ? !1 : e[0]?.data?.[t] === !0;
}
var Ue = class {
    _console = console;
    handleError(t) {
      this._console.error("ERROR", t);
    }
  },
  LE = new I("", {
    providedIn: "root",
    factory: () => {
      let e = m(J),
        t = m(Ue);
      return (n) => e.runOutsideAngular(() => t.handleError(n));
    },
  }),
  ms = class {
    destroyed = !1;
    listeners = null;
    errorHandler = m(Ue, { optional: !0 });
    destroyRef = m(It);
    constructor() {
      this.destroyRef.onDestroy(() => {
        ((this.destroyed = !0), (this.listeners = null));
      });
    }
    subscribe(t) {
      if (this.destroyed) throw new v(953, !1);
      return (
        (this.listeners ??= []).push(t),
        {
          unsubscribe: () => {
            let n = this.listeners?.indexOf(t);
            n !== void 0 && n !== -1 && this.listeners?.splice(n, 1);
          },
        }
      );
    }
    emit(t) {
      if (this.destroyed) {
        console.warn(yo(953, !1));
        return;
      }
      if (this.listeners === null) return;
      let n = A(null);
      try {
        for (let r of this.listeners)
          try {
            r(t);
          } catch (o) {
            this.errorHandler?.handleError(o);
          }
      } finally {
        A(n);
      }
    }
  };
function c1(e) {
  return new ms();
}
function Dh(e, t) {
  return sp(e, t);
}
function jE(e) {
  return sp(Rs, e);
}
var bt = ((Dh.required = jE), Dh);
function VE() {
  return wr(we(), C());
}
function wr(e, t) {
  return new tt(Xe(e, t));
}
var tt = (() => {
  class e {
    nativeElement;
    constructor(n) {
      this.nativeElement = n;
    }
    static __NG_ELEMENT_ID__ = VE;
  }
  return e;
})();
function gg(e) {
  return e instanceof tt ? e.nativeElement : e;
}
function u1(e) {
  return typeof e == "function" && e[re] !== void 0;
}
function sr(e, t) {
  let n = Wa(e, t?.equal),
    r = n[re];
  return (
    (n.set = (o) => tn(r, o)),
    (n.update = (o) => hi(r, o)),
    (n.asReadonly = Hs.bind(n)),
    n
  );
}
function Hs() {
  let e = this[re];
  if (e.readonlyFn === void 0) {
    let t = () => this();
    ((t[re] = e), (e.readonlyFn = t));
  }
  return e.readonlyFn;
}
function BE() {
  return this._results[Symbol.iterator]();
}
var Yc = class {
  _emitDistinctChangesOnly;
  dirty = !0;
  _onDirty = void 0;
  _results = [];
  _changesDetected = !1;
  _changes = void 0;
  length = 0;
  first = void 0;
  last = void 0;
  get changes() {
    return (this._changes ??= new X());
  }
  constructor(t = !1) {
    this._emitDistinctChangesOnly = t;
  }
  get(t) {
    return this._results[t];
  }
  map(t) {
    return this._results.map(t);
  }
  filter(t) {
    return this._results.filter(t);
  }
  find(t) {
    return this._results.find(t);
  }
  reduce(t, n) {
    return this._results.reduce(t, n);
  }
  forEach(t) {
    this._results.forEach(t);
  }
  some(t) {
    return this._results.some(t);
  }
  toArray() {
    return this._results.slice();
  }
  toString() {
    return this._results.toString();
  }
  reset(t, n) {
    this.dirty = !1;
    let r = Tw(t);
    (this._changesDetected = !Sw(this._results, r, n)) &&
      ((this._results = r),
      (this.length = r.length),
      (this.last = r[this.length - 1]),
      (this.first = r[0]));
  }
  notifyOnChanges() {
    this._changes !== void 0 &&
      (this._changesDetected || !this._emitDistinctChangesOnly) &&
      this._changes.next(this);
  }
  onDirty(t) {
    this._onDirty = t;
  }
  setDirty() {
    ((this.dirty = !0), this._onDirty?.());
  }
  destroy() {
    this._changes !== void 0 &&
      (this._changes.complete(), this._changes.unsubscribe());
  }
  [Symbol.iterator] = BE;
};
function mg(e) {
  return (e.flags & 128) === 128;
}
var yg = (function (e) {
    return (
      (e[(e.OnPush = 0)] = "OnPush"),
      (e[(e.Default = 1)] = "Default"),
      e
    );
  })(yg || {}),
  vg = new Map(),
  UE = 0;
function $E() {
  return UE++;
}
function HE(e) {
  vg.set(e[Fs], e);
}
function Qc(e) {
  vg.delete(e[Fs]);
}
var wh = "__ngContext__";
function Ht(e, t) {
  Bt(t) ? ((e[wh] = t[Fs]), HE(t)) : (e[wh] = t);
}
function Dg(e) {
  return Eg(e[ao]);
}
function wg(e) {
  return Eg(e[Be]);
}
function Eg(e) {
  for (; e !== null && !wt(e); ) e = e[Be];
  return e;
}
var Kc;
function Ig(e) {
  Kc = e;
}
function vl() {
  if (Kc !== void 0) return Kc;
  if (typeof document < "u") return document;
  throw new v(210, !1);
}
var Dl = new I("", { providedIn: "root", factory: () => zE }),
  zE = "ng",
  wl = new I(""),
  bn = new I("", { providedIn: "platform", factory: () => "unknown" });
var l1 = new I(""),
  El = new I("", {
    providedIn: "root",
    factory: () =>
      vl().body?.querySelector("[ngCspNonce]")?.getAttribute("ngCspNonce") ||
      null,
  });
var qE = "h",
  GE = "b";
var Cg = !1,
  WE = new I("", { providedIn: "root", factory: () => Cg });
var Il = (function (e) {
    return (
      (e[(e.CHANGE_DETECTION = 0)] = "CHANGE_DETECTION"),
      (e[(e.AFTER_NEXT_RENDER = 1)] = "AFTER_NEXT_RENDER"),
      e
    );
  })(Il || {}),
  Mn = new I(""),
  Eh = new Set();
function zt(e) {
  Eh.has(e) ||
    (Eh.add(e),
    performance?.mark?.("mark_feature_usage", { detail: { feature: e } }));
}
var zs = (() => {
  class e {
    view;
    node;
    constructor(n, r) {
      ((this.view = n), (this.node = r));
    }
    static __NG_ELEMENT_ID__ = ZE;
  }
  return e;
})();
function ZE() {
  return new zs(C(), we());
}
var Qn = (function (e) {
    return (
      (e[(e.EarlyRead = 0)] = "EarlyRead"),
      (e[(e.Write = 1)] = "Write"),
      (e[(e.MixedReadWrite = 2)] = "MixedReadWrite"),
      (e[(e.Read = 3)] = "Read"),
      e
    );
  })(Qn || {}),
  Cl = (() => {
    class e {
      impl = null;
      execute() {
        this.impl?.execute();
      }
      static ɵprov = E({
        token: e,
        providedIn: "root",
        factory: () => new e(),
      });
    }
    return e;
  })(),
  bg = [Qn.EarlyRead, Qn.Write, Qn.MixedReadWrite, Qn.Read],
  Mg = (() => {
    class e {
      ngZone = m(J);
      scheduler = m(mt);
      errorHandler = m(Ue, { optional: !0 });
      sequences = new Set();
      deferredRegistrations = new Set();
      executing = !1;
      constructor() {
        m(Mn, { optional: !0 });
      }
      execute() {
        let n = this.sequences.size > 0;
        (n && $(16), (this.executing = !0));
        for (let r of bg)
          for (let o of this.sequences)
            if (!(o.erroredOrDestroyed || !o.hooks[r]))
              try {
                o.pipelinedValue = this.ngZone.runOutsideAngular(() =>
                  this.maybeTrace(() => {
                    let i = o.hooks[r];
                    return i(o.pipelinedValue);
                  }, o.snapshot),
                );
              } catch (i) {
                ((o.erroredOrDestroyed = !0),
                  this.errorHandler?.handleError(i));
              }
        this.executing = !1;
        for (let r of this.sequences)
          (r.afterRun(), r.once && (this.sequences.delete(r), r.destroy()));
        for (let r of this.deferredRegistrations) this.sequences.add(r);
        (this.deferredRegistrations.size > 0 && this.scheduler.notify(7),
          this.deferredRegistrations.clear(),
          n && $(17));
      }
      register(n) {
        let { view: r } = n;
        r !== void 0
          ? ((r[Jn] ??= []).push(n), Dr(r), (r[_] |= 8192))
          : this.executing
            ? this.deferredRegistrations.add(n)
            : this.addSequence(n);
      }
      addSequence(n) {
        (this.sequences.add(n), this.scheduler.notify(7));
      }
      unregister(n) {
        this.executing && this.sequences.has(n)
          ? ((n.erroredOrDestroyed = !0),
            (n.pipelinedValue = void 0),
            (n.once = !0))
          : (this.sequences.delete(n), this.deferredRegistrations.delete(n));
      }
      maybeTrace(n, r) {
        return r ? r.run(Il.AFTER_NEXT_RENDER, n) : n();
      }
      static ɵprov = E({
        token: e,
        providedIn: "root",
        factory: () => new e(),
      });
    }
    return e;
  })(),
  ys = class {
    impl;
    hooks;
    view;
    once;
    snapshot;
    erroredOrDestroyed = !1;
    pipelinedValue = void 0;
    unregisterOnDestroy;
    constructor(t, n, r, o, i, s = null) {
      ((this.impl = t),
        (this.hooks = n),
        (this.view = r),
        (this.once = o),
        (this.snapshot = s),
        (this.unregisterOnDestroy = i?.onDestroy(() => this.destroy())));
    }
    afterRun() {
      ((this.erroredOrDestroyed = !1),
        (this.pipelinedValue = void 0),
        this.snapshot?.dispose(),
        (this.snapshot = null));
    }
    destroy() {
      (this.impl.unregister(this), this.unregisterOnDestroy?.());
      let t = this.view?.[Jn];
      t && (this.view[Jn] = t.filter((n) => n !== this));
    }
  };
function YE(e, t) {
  !t?.injector && gr(YE);
  let n = t?.injector ?? m(ce);
  return (zt("NgAfterRender"), Sg(e, n, t, !1));
}
function bl(e, t) {
  !t?.injector && gr(bl);
  let n = t?.injector ?? m(ce);
  return (zt("NgAfterNextRender"), Sg(e, n, t, !0));
}
function QE(e, t) {
  if (e instanceof Function) {
    let n = [void 0, void 0, void 0, void 0];
    return ((n[t] = e), n);
  } else return [e.earlyRead, e.write, e.mixedReadWrite, e.read];
}
function Sg(e, t, n, r) {
  let o = t.get(Cl);
  o.impl ??= t.get(Mg);
  let i = t.get(Mn, null, { optional: !0 }),
    s = n?.phase ?? Qn.MixedReadWrite,
    a = n?.manualCleanup !== !0 ? t.get(It) : null,
    c = t.get(zs, null, { optional: !0 }),
    u = new ys(o.impl, QE(e, s), c?.view, r, a, i?.snapshot(null));
  return (o.impl.register(u), u);
}
var KE = () => null;
function Tg(e, t, n = !1) {
  return KE(e, t, n);
}
function _g(e, t) {
  let n = e.contentQueries;
  if (n !== null) {
    let r = A(null);
    try {
      for (let o = 0; o < n.length; o += 2) {
        let i = n[o],
          s = n[o + 1];
        if (s !== -1) {
          let a = e.data[s];
          (Bs(i), a.contentQueries(2, t[s], s));
        }
      }
    } finally {
      A(r);
    }
  }
}
function Jc(e, t, n) {
  Bs(0);
  let r = A(null);
  try {
    t(e, n);
  } finally {
    A(r);
  }
}
function Ml(e, t, n) {
  if (nl(t)) {
    let r = A(null);
    try {
      let o = t.directiveStart,
        i = t.directiveEnd;
      for (let s = o; s < i; s++) {
        let a = e.data[s];
        if (a.contentQueries) {
          let c = n[s];
          a.contentQueries(1, c, s);
        }
      }
    } finally {
      A(r);
    }
  }
}
var Ke = (function (e) {
    return (
      (e[(e.Emulated = 0)] = "Emulated"),
      (e[(e.None = 2)] = "None"),
      (e[(e.ShadowDom = 3)] = "ShadowDom"),
      e
    );
  })(Ke || {}),
  Hi;
function JE() {
  if (Hi === void 0 && ((Hi = null), ft.trustedTypes))
    try {
      Hi = ft.trustedTypes.createPolicy("angular", {
        createHTML: (e) => e,
        createScript: (e) => e,
        createScriptURL: (e) => e,
      });
    } catch {}
  return Hi;
}
function qs(e) {
  return JE()?.createHTML(e) || e;
}
var zi;
function xg() {
  if (zi === void 0 && ((zi = null), ft.trustedTypes))
    try {
      zi = ft.trustedTypes.createPolicy("angular#unsafe-bypass", {
        createHTML: (e) => e,
        createScript: (e) => e,
        createScriptURL: (e) => e,
      });
    } catch {}
  return zi;
}
function Ih(e) {
  return xg()?.createHTML(e) || e;
}
function Ch(e) {
  return xg()?.createScriptURL(e) || e;
}
var yt = class {
    changingThisBreaksApplicationSecurity;
    constructor(t) {
      this.changingThisBreaksApplicationSecurity = t;
    }
    toString() {
      return `SafeValue must use [property]=binding: ${this.changingThisBreaksApplicationSecurity} (see ${op})`;
    }
  },
  Xc = class extends yt {
    getTypeName() {
      return "HTML";
    }
  },
  eu = class extends yt {
    getTypeName() {
      return "Style";
    }
  },
  tu = class extends yt {
    getTypeName() {
      return "Script";
    }
  },
  nu = class extends yt {
    getTypeName() {
      return "URL";
    }
  },
  ru = class extends yt {
    getTypeName() {
      return "ResourceURL";
    }
  };
function xe(e) {
  return e instanceof yt ? e.changingThisBreaksApplicationSecurity : e;
}
function Mt(e, t) {
  let n = XE(e);
  if (n != null && n !== t) {
    if (n === "ResourceURL" && t === "URL") return !0;
    throw new Error(`Required a safe ${t}, got a ${n} (see ${op})`);
  }
  return n === t;
}
function XE(e) {
  return (e instanceof yt && e.getTypeName()) || null;
}
function Ng(e) {
  return new Xc(e);
}
function Rg(e) {
  return new eu(e);
}
function Ag(e) {
  return new tu(e);
}
function Og(e) {
  return new nu(e);
}
function kg(e) {
  return new ru(e);
}
function Pg(e) {
  let t = new iu(e);
  return eI() ? new ou(t) : t;
}
var ou = class {
    inertDocumentHelper;
    constructor(t) {
      this.inertDocumentHelper = t;
    }
    getInertBodyElement(t) {
      t = "<body><remove></remove>" + t;
      try {
        let n = new window.DOMParser().parseFromString(qs(t), "text/html").body;
        return n === null
          ? this.inertDocumentHelper.getInertBodyElement(t)
          : (n.firstChild?.remove(), n);
      } catch {
        return null;
      }
    }
  },
  iu = class {
    defaultDoc;
    inertDocument;
    constructor(t) {
      ((this.defaultDoc = t),
        (this.inertDocument =
          this.defaultDoc.implementation.createHTMLDocument(
            "sanitization-inert",
          )));
    }
    getInertBodyElement(t) {
      let n = this.inertDocument.createElement("template");
      return ((n.innerHTML = qs(t)), n);
    }
  };
function eI() {
  try {
    return !!new window.DOMParser().parseFromString(qs(""), "text/html");
  } catch {
    return !1;
  }
}
var tI = /^(?!javascript:)(?:[a-z0-9+.-]+:|[^&:\/?#]*(?:[\/?#]|$))/i;
function Mo(e) {
  return ((e = String(e)), e.match(tI) ? e : "unsafe:" + e);
}
function St(e) {
  let t = {};
  for (let n of e.split(",")) t[n] = !0;
  return t;
}
function So(...e) {
  let t = {};
  for (let n of e) for (let r in n) n.hasOwnProperty(r) && (t[r] = !0);
  return t;
}
var Fg = St("area,br,col,hr,img,wbr"),
  Lg = St("colgroup,dd,dt,li,p,tbody,td,tfoot,th,thead,tr"),
  jg = St("rp,rt"),
  nI = So(jg, Lg),
  rI = So(
    Lg,
    St(
      "address,article,aside,blockquote,caption,center,del,details,dialog,dir,div,dl,figure,figcaption,footer,h1,h2,h3,h4,h5,h6,header,hgroup,hr,ins,main,map,menu,nav,ol,pre,section,summary,table,ul",
    ),
  ),
  oI = So(
    jg,
    St(
      "a,abbr,acronym,audio,b,bdi,bdo,big,br,cite,code,del,dfn,em,font,i,img,ins,kbd,label,map,mark,picture,q,ruby,rp,rt,s,samp,small,source,span,strike,strong,sub,sup,time,track,tt,u,var,video",
    ),
  ),
  su = So(Fg, rI, oI, nI),
  Sl = St("background,cite,href,itemtype,longdesc,poster,src,xlink:href"),
  iI = St(
    "abbr,accesskey,align,alt,autoplay,axis,bgcolor,border,cellpadding,cellspacing,class,clear,color,cols,colspan,compact,controls,coords,datetime,default,dir,download,face,headers,height,hidden,hreflang,hspace,ismap,itemscope,itemprop,kind,label,lang,language,loop,media,muted,nohref,nowrap,open,preload,rel,rev,role,rows,rowspan,rules,scope,scrolling,shape,size,sizes,span,srclang,srcset,start,summary,tabindex,target,title,translate,type,usemap,valign,value,vspace,width",
  ),
  sI = St(
    "aria-activedescendant,aria-atomic,aria-autocomplete,aria-busy,aria-checked,aria-colcount,aria-colindex,aria-colspan,aria-controls,aria-current,aria-describedby,aria-details,aria-disabled,aria-dropeffect,aria-errormessage,aria-expanded,aria-flowto,aria-grabbed,aria-haspopup,aria-hidden,aria-invalid,aria-keyshortcuts,aria-label,aria-labelledby,aria-level,aria-live,aria-modal,aria-multiline,aria-multiselectable,aria-orientation,aria-owns,aria-placeholder,aria-posinset,aria-pressed,aria-readonly,aria-relevant,aria-required,aria-roledescription,aria-rowcount,aria-rowindex,aria-rowspan,aria-selected,aria-setsize,aria-sort,aria-valuemax,aria-valuemin,aria-valuenow,aria-valuetext",
  ),
  Vg = So(Sl, iI, sI),
  aI = St("script,style,template"),
  au = class {
    sanitizedSomething = !1;
    buf = [];
    sanitizeChildren(t) {
      let n = t.firstChild,
        r = !0,
        o = [];
      for (; n; ) {
        if (
          (n.nodeType === Node.ELEMENT_NODE
            ? (r = this.startElement(n))
            : n.nodeType === Node.TEXT_NODE
              ? this.chars(n.nodeValue)
              : (this.sanitizedSomething = !0),
          r && n.firstChild)
        ) {
          (o.push(n), (n = lI(n)));
          continue;
        }
        for (; n; ) {
          n.nodeType === Node.ELEMENT_NODE && this.endElement(n);
          let i = uI(n);
          if (i) {
            n = i;
            break;
          }
          n = o.pop();
        }
      }
      return this.buf.join("");
    }
    startElement(t) {
      let n = bh(t).toLowerCase();
      if (!su.hasOwnProperty(n))
        return ((this.sanitizedSomething = !0), !aI.hasOwnProperty(n));
      (this.buf.push("<"), this.buf.push(n));
      let r = t.attributes;
      for (let o = 0; o < r.length; o++) {
        let i = r.item(o),
          s = i.name,
          a = s.toLowerCase();
        if (!Vg.hasOwnProperty(a)) {
          this.sanitizedSomething = !0;
          continue;
        }
        let c = i.value;
        (Sl[a] && (c = Mo(c)), this.buf.push(" ", s, '="', Mh(c), '"'));
      }
      return (this.buf.push(">"), !0);
    }
    endElement(t) {
      let n = bh(t).toLowerCase();
      su.hasOwnProperty(n) &&
        !Fg.hasOwnProperty(n) &&
        (this.buf.push("</"), this.buf.push(n), this.buf.push(">"));
    }
    chars(t) {
      this.buf.push(Mh(t));
    }
  };
function cI(e, t) {
  return (
    (e.compareDocumentPosition(t) & Node.DOCUMENT_POSITION_CONTAINED_BY) !==
    Node.DOCUMENT_POSITION_CONTAINED_BY
  );
}
function uI(e) {
  let t = e.nextSibling;
  if (t && e !== t.previousSibling) throw Bg(t);
  return t;
}
function lI(e) {
  let t = e.firstChild;
  if (t && cI(e, t)) throw Bg(t);
  return t;
}
function bh(e) {
  let t = e.nodeName;
  return typeof t == "string" ? t : "FORM";
}
function Bg(e) {
  return new Error(
    `Failed to sanitize html because the element is clobbered: ${e.outerHTML}`,
  );
}
var dI = /[\uD800-\uDBFF][\uDC00-\uDFFF]/g,
  fI = /([^\#-~ |!])/g;
function Mh(e) {
  return e
    .replace(/&/g, "&amp;")
    .replace(dI, function (t) {
      let n = t.charCodeAt(0),
        r = t.charCodeAt(1);
      return "&#" + ((n - 55296) * 1024 + (r - 56320) + 65536) + ";";
    })
    .replace(fI, function (t) {
      return "&#" + t.charCodeAt(0) + ";";
    })
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}
var qi;
function Tl(e, t) {
  let n = null;
  try {
    qi = qi || Pg(e);
    let r = t ? String(t) : "";
    n = qi.getInertBodyElement(r);
    let o = 5,
      i = r;
    do {
      if (o === 0)
        throw new Error(
          "Failed to sanitize html because the input is unstable",
        );
      (o--, (r = i), (i = n.innerHTML), (n = qi.getInertBodyElement(r)));
    } while (r !== i);
    let a = new au().sanitizeChildren(cu(n) || n);
    return qs(a);
  } finally {
    if (n) {
      let r = cu(n) || n;
      for (; r.firstChild; ) r.firstChild.remove();
    }
  }
}
function cu(e) {
  return "content" in e && hI(e) ? e.content : null;
}
function hI(e) {
  return e.nodeType === Node.ELEMENT_NODE && e.nodeName === "TEMPLATE";
}
var $e = (function (e) {
  return (
    (e[(e.NONE = 0)] = "NONE"),
    (e[(e.HTML = 1)] = "HTML"),
    (e[(e.STYLE = 2)] = "STYLE"),
    (e[(e.SCRIPT = 3)] = "SCRIPT"),
    (e[(e.URL = 4)] = "URL"),
    (e[(e.RESOURCE_URL = 5)] = "RESOURCE_URL"),
    e
  );
})($e || {});
function d1(e) {
  let t = _l();
  return t
    ? Ih(t.sanitize($e.HTML, e) || "")
    : Mt(e, "HTML")
      ? Ih(xe(e))
      : Tl(vl(), Ut(e));
}
function pI(e) {
  let t = _l();
  return t ? t.sanitize($e.URL, e) || "" : Mt(e, "URL") ? xe(e) : Mo(Ut(e));
}
function gI(e) {
  let t = _l();
  if (t) return Ch(t.sanitize($e.RESOURCE_URL, e) || "");
  if (Mt(e, "ResourceURL")) return Ch(xe(e));
  throw new v(904, !1);
}
function mI(e, t) {
  return (t === "src" &&
    (e === "embed" ||
      e === "frame" ||
      e === "iframe" ||
      e === "media" ||
      e === "script")) ||
    (t === "href" && (e === "base" || e === "link"))
    ? gI
    : pI;
}
function Ug(e, t, n) {
  return mI(t, n)(e);
}
function _l() {
  let e = C();
  return e && e[ht].sanitizer;
}
var yI = /^>|^->|<!--|-->|--!>|<!-$/g,
  vI = /(<|>)/g,
  DI = "\u200B$1\u200B";
function wI(e) {
  return e.replace(yI, (t) => t.replace(vI, DI));
}
function f1(e) {
  return e.ownerDocument.defaultView;
}
function $g(e) {
  return e instanceof Function ? e() : e;
}
function EI(e, t, n) {
  let r = e.length;
  for (;;) {
    let o = e.indexOf(t, n);
    if (o === -1) return o;
    if (o === 0 || e.charCodeAt(o - 1) <= 32) {
      let i = t.length;
      if (o + i === r || e.charCodeAt(o + i) <= 32) return o;
    }
    n = o + 1;
  }
}
var Hg = "ng-template";
function II(e, t, n, r) {
  let o = 0;
  if (r) {
    for (; o < t.length && typeof t[o] == "string"; o += 2)
      if (t[o] === "class" && EI(t[o + 1].toLowerCase(), n, 0) !== -1)
        return !0;
  } else if (xl(e)) return !1;
  if (((o = t.indexOf(1, o)), o > -1)) {
    let i;
    for (; ++o < t.length && typeof (i = t[o]) == "string"; )
      if (i.toLowerCase() === n) return !0;
  }
  return !1;
}
function xl(e) {
  return e.type === 4 && e.value !== Hg;
}
function CI(e, t, n) {
  let r = e.type === 4 && !n ? Hg : e.value;
  return t === r;
}
function bI(e, t, n) {
  let r = 4,
    o = e.attrs,
    i = o !== null ? TI(o) : 0,
    s = !1;
  for (let a = 0; a < t.length; a++) {
    let c = t[a];
    if (typeof c == "number") {
      if (!s && !Ve(r) && !Ve(c)) return !1;
      if (s && Ve(c)) continue;
      ((s = !1), (r = c | (r & 1)));
      continue;
    }
    if (!s)
      if (r & 4) {
        if (
          ((r = 2 | (r & 1)),
          (c !== "" && !CI(e, c, n)) || (c === "" && t.length === 1))
        ) {
          if (Ve(r)) return !1;
          s = !0;
        }
      } else if (r & 8) {
        if (o === null || !II(e, o, c, n)) {
          if (Ve(r)) return !1;
          s = !0;
        }
      } else {
        let u = t[++a],
          l = MI(c, o, xl(e), n);
        if (l === -1) {
          if (Ve(r)) return !1;
          s = !0;
          continue;
        }
        if (u !== "") {
          let d;
          if (
            (l > i ? (d = "") : (d = o[l + 1].toLowerCase()), r & 2 && u !== d)
          ) {
            if (Ve(r)) return !1;
            s = !0;
          }
        }
      }
  }
  return Ve(r) || s;
}
function Ve(e) {
  return (e & 1) === 0;
}
function MI(e, t, n, r) {
  if (t === null) return -1;
  let o = 0;
  if (r || !n) {
    let i = !1;
    for (; o < t.length; ) {
      let s = t[o];
      if (s === e) return o;
      if (s === 3 || s === 6) i = !0;
      else if (s === 1 || s === 2) {
        let a = t[++o];
        for (; typeof a == "string"; ) a = t[++o];
        continue;
      } else {
        if (s === 4) break;
        if (s === 0) {
          o += 4;
          continue;
        }
      }
      o += i ? 1 : 2;
    }
    return -1;
  } else return _I(t, e);
}
function zg(e, t, n = !1) {
  for (let r = 0; r < t.length; r++) if (bI(e, t[r], n)) return !0;
  return !1;
}
function SI(e) {
  let t = e.attrs;
  if (t != null) {
    let n = t.indexOf(5);
    if ((n & 1) === 0) return t[n + 1];
  }
  return null;
}
function TI(e) {
  for (let t = 0; t < e.length; t++) {
    let n = e[t];
    if (Xp(n)) return t;
  }
  return e.length;
}
function _I(e, t) {
  let n = e.indexOf(4);
  if (n > -1)
    for (n++; n < e.length; ) {
      let r = e[n];
      if (typeof r == "number") return -1;
      if (r === t) return n;
      n++;
    }
  return -1;
}
function xI(e, t) {
  e: for (let n = 0; n < t.length; n++) {
    let r = t[n];
    if (e.length === r.length) {
      for (let o = 0; o < e.length; o++) if (e[o] !== r[o]) continue e;
      return !0;
    }
  }
  return !1;
}
function Sh(e, t) {
  return e ? ":not(" + t.trim() + ")" : t;
}
function NI(e) {
  let t = e[0],
    n = 1,
    r = 2,
    o = "",
    i = !1;
  for (; n < e.length; ) {
    let s = e[n];
    if (typeof s == "string") {
      if (r & 2) {
        let a = e[++n];
        o += "[" + s + (a.length > 0 ? '="' + a + '"' : "") + "]";
      } else r & 8 ? (o += "." + s) : r & 4 && (o += " " + s);
    } else
      (o !== "" && !Ve(s) && ((t += Sh(i, o)), (o = "")),
        (r = s),
        (i = i || !Ve(r)));
    n++;
  }
  return (o !== "" && (t += Sh(i, o)), t);
}
function RI(e) {
  return e.map(NI).join(",");
}
function AI(e) {
  let t = [],
    n = [],
    r = 1,
    o = 2;
  for (; r < e.length; ) {
    let i = e[r];
    if (typeof i == "string")
      o === 2 ? i !== "" && t.push(i, e[++r]) : o === 8 && n.push(i);
    else {
      if (!Ve(o)) break;
      o = i;
    }
    r++;
  }
  return (n.length && t.push(1, ...n), t);
}
var Se = {};
function qg(e, t) {
  return e.createText(t);
}
function Gg(e, t, n) {
  e.setValue(t, n);
}
function Wg(e, t) {
  return e.createComment(wI(t));
}
function Nl(e, t, n) {
  return e.createElement(t, n);
}
function vn(e, t, n, r, o) {
  e.insertBefore(t, n, r, o);
}
function Zg(e, t, n) {
  e.appendChild(t, n);
}
function Th(e, t, n, r, o) {
  r !== null ? vn(e, t, n, r, o) : Zg(e, t, n);
}
function Yg(e, t, n) {
  e.removeChild(null, t, n);
}
function OI(e, t, n) {
  e.setAttribute(t, "style", n);
}
function kI(e, t, n) {
  n === "" ? e.removeAttribute(t, "class") : e.setAttribute(t, "class", n);
}
function Qg(e, t, n) {
  let { mergedAttrs: r, classes: o, styles: i } = n;
  (r !== null && yE(e, t, r),
    o !== null && kI(e, t, o),
    i !== null && OI(e, t, i));
}
function Rl(e, t, n, r, o, i, s, a, c, u, l) {
  let d = Y + r,
    h = d + o,
    f = PI(d, h),
    p = typeof u == "function" ? u() : u;
  return (f[N] = {
    type: e,
    blueprint: f,
    template: n,
    queries: null,
    viewQuery: a,
    declTNode: t,
    data: f.slice().fill(null, d),
    bindingStartIndex: d,
    expandoStartIndex: h,
    hostBindingOpCodes: null,
    firstCreatePass: !0,
    firstUpdatePass: !0,
    staticViewQueries: !1,
    staticContentQueries: !1,
    preOrderHooks: null,
    preOrderCheckHooks: null,
    contentHooks: null,
    contentCheckHooks: null,
    viewHooks: null,
    viewCheckHooks: null,
    destroyHooks: null,
    cleanup: null,
    contentQueries: null,
    components: null,
    directiveRegistry: typeof i == "function" ? i() : i,
    pipeRegistry: typeof s == "function" ? s() : s,
    firstChild: null,
    schemas: c,
    consts: p,
    incompleteFirstPass: !1,
    ssrId: l,
  });
}
function PI(e, t) {
  let n = [];
  for (let r = 0; r < t; r++) n.push(r < e ? null : Se);
  return n;
}
function FI(e) {
  let t = e.tView;
  return t === null || t.incompleteFirstPass
    ? (e.tView = Rl(
        1,
        null,
        e.template,
        e.decls,
        e.vars,
        e.directiveDefs,
        e.pipeDefs,
        e.viewQuery,
        e.schemas,
        e.consts,
        e.id,
      ))
    : t;
}
function Al(e, t, n, r, o, i, s, a, c, u, l) {
  let d = t.blueprint.slice();
  return (
    (d[Dt] = o),
    (d[_] = r | 4 | 128 | 8 | 64 | 1024),
    (u !== null || (e && e[_] & 2048)) && (d[_] |= 2048),
    Fp(d),
    (d[he] = d[mr] = e),
    (d[ne] = n),
    (d[ht] = s || (e && e[ht])),
    (d[U] = a || (e && e[U])),
    (d[rr] = c || (e && e[rr]) || null),
    (d[De] = i),
    (d[Fs] = $E()),
    (d[nr] = l),
    (d[xp] = u),
    (d[be] = t.type == 2 ? e[be] : d),
    d
  );
}
function LI(e, t, n) {
  let r = Xe(t, e),
    o = FI(n),
    i = e[ht].rendererFactory,
    s = Ol(
      e,
      Al(
        e,
        o,
        null,
        Kg(n),
        r,
        t,
        null,
        i.createRenderer(r, n),
        null,
        null,
        null,
      ),
    );
  return (e[t.index] = s);
}
function Kg(e) {
  let t = 16;
  return (e.signals ? (t = 4096) : e.onPush && (t = 64), t);
}
function To(e, t, n, r) {
  if (n === 0) return -1;
  let o = t.length;
  for (let i = 0; i < n; i++)
    (t.push(r), e.blueprint.push(r), e.data.push(null));
  return o;
}
function Ol(e, t) {
  return (e[ao] ? (e[uh][Be] = t) : (e[ao] = t), (e[uh] = t), t);
}
function kl(e = 1) {
  Jg(Q(), C(), Et() + e, !1);
}
function Jg(e, t, n, r) {
  if (!r)
    if ((t[_] & 3) === 3) {
      let i = e.preOrderCheckHooks;
      i !== null && Zi(t, i, n);
    } else {
      let i = e.preOrderHooks;
      i !== null && Yi(t, i, 0, n);
    }
  mn(n);
}
var Gs = (function (e) {
  return (
    (e[(e.None = 0)] = "None"),
    (e[(e.SignalBased = 1)] = "SignalBased"),
    (e[(e.HasDecoratorInputTransform = 2)] = "HasDecoratorInputTransform"),
    e
  );
})(Gs || {});
function uu(e, t, n, r) {
  let o = A(null);
  try {
    let [i, s, a] = e.inputs[n],
      c = null;
    ((s & Gs.SignalBased) !== 0 && (c = t[i][re]),
      c !== null && c.transformFn !== void 0
        ? (r = c.transformFn(r))
        : a !== null && (r = a.call(t, r)),
      e.setInput !== null ? e.setInput(t, c, r, n, i) : Rp(t, c, i, r));
  } finally {
    A(o);
  }
}
function Xg(e, t, n, r, o) {
  let i = Et(),
    s = r & 2;
  try {
    (mn(-1), s && t.length > Y && Jg(e, t, Y, !1), $(s ? 2 : 0, o), n(r, o));
  } finally {
    (mn(i), $(s ? 3 : 1, o));
  }
}
function Ws(e, t, n) {
  (HI(e, t, n), (n.flags & 64) === 64 && zI(e, t, n));
}
function Pl(e, t, n = Xe) {
  let r = t.localNames;
  if (r !== null) {
    let o = t.index + 1;
    for (let i = 0; i < r.length; i += 2) {
      let s = r[i + 1],
        a = s === -1 ? n(t, e) : e[s];
      e[o++] = a;
    }
  }
}
function jI(e, t, n, r) {
  let i = r.get(WE, Cg) || n === Ke.ShadowDom,
    s = e.selectRootElement(t, i);
  return (VI(s), s);
}
function VI(e) {
  BI(e);
}
var BI = () => null;
function UI(e) {
  return e === "class"
    ? "className"
    : e === "for"
      ? "htmlFor"
      : e === "formaction"
        ? "formAction"
        : e === "innerHtml"
          ? "innerHTML"
          : e === "readonly"
            ? "readOnly"
            : e === "tabindex"
              ? "tabIndex"
              : e;
}
function Zs(e, t, n, r, o, i, s, a) {
  if (!a && jl(t, e, n, r, o)) {
    yr(t) && $I(n, t.index);
    return;
  }
  if (t.type & 3) {
    let c = Xe(t, n);
    ((r = UI(r)),
      (o = s != null ? s(o, t.value || "", r) : o),
      i.setProperty(c, r, o));
  } else t.type & 12;
}
function $I(e, t) {
  let n = Ye(t, e);
  n[_] & 16 || (n[_] |= 64);
}
function HI(e, t, n) {
  let r = n.directiveStart,
    o = n.directiveEnd;
  (yr(n) && LI(t, n, e.data[r + n.componentOffset]),
    e.firstCreatePass || hs(n, t));
  let i = n.initialInputs;
  for (let s = r; s < o; s++) {
    let a = e.data[s],
      c = uo(t, e, s, n);
    if ((Ht(c, t), i !== null && WI(t, s - r, c, a, n, i), Ze(a))) {
      let u = Ye(n.index, t);
      u[ne] = uo(t, e, s, n);
    }
  }
}
function zI(e, t, n) {
  let r = n.directiveStart,
    o = n.directiveEnd,
    i = n.index,
    s = aE();
  try {
    mn(i);
    for (let a = r; a < o; a++) {
      let c = e.data[a],
        u = t[a];
      (Uc(a),
        (c.hostBindings !== null || c.hostVars !== 0 || c.hostAttrs !== null) &&
          qI(c, u));
    }
  } finally {
    (mn(-1), Uc(s));
  }
}
function qI(e, t) {
  e.hostBindings !== null && e.hostBindings(1, t);
}
function Fl(e, t) {
  let n = e.directiveRegistry,
    r = null;
  if (n)
    for (let o = 0; o < n.length; o++) {
      let i = n[o];
      zg(t, i.selectors, !1) && ((r ??= []), Ze(i) ? r.unshift(i) : r.push(i));
    }
  return r;
}
function GI(e, t, n, r, o, i) {
  let s = Xe(e, t);
  Ll(t[U], s, i, e.value, n, r, o);
}
function Ll(e, t, n, r, o, i, s) {
  if (i == null) e.removeAttribute(t, o, n);
  else {
    let a = s == null ? Ut(i) : s(i, r || "", o);
    e.setAttribute(t, o, a, n);
  }
}
function WI(e, t, n, r, o, i) {
  let s = i[t];
  if (s !== null)
    for (let a = 0; a < s.length; a += 2) {
      let c = s[a],
        u = s[a + 1];
      uu(r, n, c, u);
    }
}
function ZI(e, t) {
  let n = e[rr],
    r = n ? n.get(Ue, null) : null;
  r && r.handleError(t);
}
function jl(e, t, n, r, o) {
  let i = e.inputs?.[r],
    s = e.hostDirectiveInputs?.[r],
    a = !1;
  if (s)
    for (let c = 0; c < s.length; c += 2) {
      let u = s[c],
        l = s[c + 1],
        d = t.data[u];
      (uu(d, n[u], l, o), (a = !0));
    }
  if (i)
    for (let c of i) {
      let u = n[c],
        l = t.data[c];
      (uu(l, u, r, o), (a = !0));
    }
  return a;
}
function YI(e, t) {
  let n = Ye(t, e),
    r = n[N];
  QI(r, n);
  let o = n[Dt];
  (o !== null && n[nr] === null && (n[nr] = Tg(o, n[rr])),
    $(18),
    Vl(r, n, n[ne]),
    $(19, n[ne]));
}
function QI(e, t) {
  for (let n = t.length; n < e.blueprint.length; n++) t.push(e.blueprint[n]);
}
function Vl(e, t, n) {
  ll(t);
  try {
    let r = e.viewQuery;
    r !== null && Jc(1, r, n);
    let o = e.template;
    (o !== null && Xg(e, t, o, 1, n),
      e.firstCreatePass && (e.firstCreatePass = !1),
      t[pt]?.finishViewCreation(e),
      e.staticContentQueries && _g(e, t),
      e.staticViewQueries && Jc(2, e.viewQuery, n));
    let i = e.components;
    i !== null && KI(t, i);
  } catch (r) {
    throw (
      e.firstCreatePass &&
        ((e.incompleteFirstPass = !0), (e.firstCreatePass = !1)),
      r
    );
  } finally {
    ((t[_] &= -5), dl());
  }
}
function KI(e, t) {
  for (let n = 0; n < t.length; n++) YI(e, t[n]);
}
function _o(e, t, n, r) {
  let o = A(null);
  try {
    let i = t.tView,
      a = e[_] & 4096 ? 4096 : 16,
      c = Al(
        e,
        i,
        n,
        a,
        null,
        t,
        null,
        null,
        r?.injector ?? null,
        r?.embeddedViewInjector ?? null,
        r?.dehydratedView ?? null,
      ),
      u = e[t.index];
    c[fn] = u;
    let l = e[pt];
    return (l !== null && (c[pt] = l.createEmbeddedView(i)), Vl(i, c, n), c);
  } finally {
    A(o);
  }
}
function ar(e, t) {
  return !t || t.firstChild === null || mg(e);
}
var lu;
function Bl(e, t) {
  return lu(e, t);
}
function JI(e) {
  lu === void 0 && (lu = e());
}
var Je = (function (e) {
  return (
    (e[(e.Important = 1)] = "Important"),
    (e[(e.DashCase = 2)] = "DashCase"),
    e
  );
})(Je || {});
function Ul(e) {
  return (e.flags & 32) === 32;
}
function Kn(e, t, n, r, o) {
  if (r != null) {
    let i,
      s = !1;
    wt(r) ? (i = r) : Bt(r) && ((s = !0), (r = r[Dt]));
    let a = Pe(r);
    (e === 0 && n !== null
      ? o == null
        ? Zg(t, n, a)
        : vn(t, n, a, o || null, !0)
      : e === 1 && n !== null
        ? vn(t, n, a, o || null, !0)
        : e === 2
          ? Yg(t, a, s)
          : e === 3 && t.destroyNode(a),
      i != null && iC(t, e, i, n, o));
  }
}
function XI(e, t) {
  (em(e, t), (t[Dt] = null), (t[De] = null));
}
function eC(e, t, n, r, o, i) {
  ((r[Dt] = o), (r[De] = t), Ks(e, r, n, 1, o, i));
}
function em(e, t) {
  (t[ht].changeDetectionScheduler?.notify(9), Ks(e, t, t[U], 2, null, null));
}
function tC(e) {
  let t = e[ao];
  if (!t) return wc(e[N], e);
  for (; t; ) {
    let n = null;
    if (Bt(t)) n = t[ao];
    else {
      let r = t[fe];
      r && (n = r);
    }
    if (!n) {
      for (; t && !t[Be] && t !== e; ) (Bt(t) && wc(t[N], t), (t = t[he]));
      (t === null && (t = e), Bt(t) && wc(t[N], t), (n = t && t[Be]));
    }
    t = n;
  }
}
function $l(e, t) {
  let n = e[or],
    r = n.indexOf(t);
  n.splice(r, 1);
}
function Ys(e, t) {
  if (Eo(t)) return;
  let n = t[U];
  (n.destroyNode && Ks(e, t, n, 3, null, null), tC(t));
}
function wc(e, t) {
  if (Eo(t)) return;
  let n = A(null);
  try {
    ((t[_] &= -129),
      (t[_] |= 256),
      t[ke] && Gr(t[ke]),
      rC(e, t),
      nC(e, t),
      t[N].type === 1 && t[U].destroy());
    let r = t[fn];
    if (r !== null && wt(t[he])) {
      r !== t[he] && $l(r, t);
      let o = t[pt];
      o !== null && o.detachView(e);
    }
    Qc(t);
  } finally {
    A(n);
  }
}
function nC(e, t) {
  let n = e.cleanup,
    r = t[ss];
  if (n !== null)
    for (let s = 0; s < n.length - 1; s += 2)
      if (typeof n[s] == "string") {
        let a = n[s + 3];
        (a >= 0 ? r[a]() : r[-a].unsubscribe(), (s += 2));
      } else {
        let a = r[n[s + 1]];
        n[s].call(a);
      }
  r !== null && (t[ss] = null);
  let o = t[Vt];
  if (o !== null) {
    t[Vt] = null;
    for (let s = 0; s < o.length; s++) {
      let a = o[s];
      a();
    }
  }
  let i = t[hn];
  if (i !== null) {
    t[hn] = null;
    for (let s of i) s.destroy();
  }
}
function rC(e, t) {
  let n;
  if (e != null && (n = e.destroyHooks) != null)
    for (let r = 0; r < n.length; r += 2) {
      let o = t[n[r]];
      if (!(o instanceof yn)) {
        let i = n[r + 1];
        if (Array.isArray(i))
          for (let s = 0; s < i.length; s += 2) {
            let a = o[i[s]],
              c = i[s + 1];
            $(4, a, c);
            try {
              c.call(a);
            } finally {
              $(5, a, c);
            }
          }
        else {
          $(4, o, i);
          try {
            i.call(o);
          } finally {
            $(5, o, i);
          }
        }
      }
    }
}
function tm(e, t, n) {
  return nm(e, t.parent, n);
}
function nm(e, t, n) {
  let r = t;
  for (; r !== null && r.type & 168; ) ((t = r), (r = t.parent));
  if (r === null) return n[Dt];
  if (yr(r)) {
    let { encapsulation: o } = e.data[r.directiveStart + r.componentOffset];
    if (o === Ke.None || o === Ke.Emulated) return null;
  }
  return Xe(r, n);
}
function rm(e, t, n) {
  return im(e, t, n);
}
function om(e, t, n) {
  return e.type & 40 ? Xe(e, n) : null;
}
var im = om,
  du;
function sm(e, t) {
  ((im = e), (du = t));
}
function Qs(e, t, n, r) {
  let o = tm(e, r, t),
    i = t[U],
    s = r.parent || t[De],
    a = rm(s, r, t);
  if (o != null)
    if (Array.isArray(n))
      for (let c = 0; c < n.length; c++) Th(i, o, n[c], a, !1);
    else Th(i, o, n, a, !1);
  du !== void 0 && du(i, r, t, n, o);
}
function eo(e, t) {
  if (t !== null) {
    let n = t.type;
    if (n & 3) return Xe(t, e);
    if (n & 4) return fu(-1, e[t.index]);
    if (n & 8) {
      let r = t.child;
      if (r !== null) return eo(e, r);
      {
        let o = e[t.index];
        return wt(o) ? fu(-1, o) : Pe(o);
      }
    } else {
      if (n & 128) return eo(e, t.next);
      if (n & 32) return Bl(t, e)() || Pe(e[t.index]);
      {
        let r = am(e, t);
        if (r !== null) {
          if (Array.isArray(r)) return r[0];
          let o = gn(e[be]);
          return eo(o, r);
        } else return eo(e, t.next);
      }
    }
  }
  return null;
}
function am(e, t) {
  if (t !== null) {
    let r = e[be][De],
      o = t.projection;
    return r.projection[o];
  }
  return null;
}
function fu(e, t) {
  let n = fe + e + 1;
  if (n < t.length) {
    let r = t[n],
      o = r[N].firstChild;
    if (o !== null) return eo(r, o);
  }
  return t[pn];
}
function Hl(e, t, n, r, o, i, s) {
  for (; n != null; ) {
    if (n.type === 128) {
      n = n.next;
      continue;
    }
    let a = r[n.index],
      c = n.type;
    if ((s && t === 0 && (a && Ht(Pe(a), r), (n.flags |= 2)), !Ul(n)))
      if (c & 8) (Hl(e, t, n.child, r, o, i, !1), Kn(t, e, o, a, i));
      else if (c & 32) {
        let u = Bl(n, r),
          l;
        for (; (l = u()); ) Kn(t, e, o, l, i);
        Kn(t, e, o, a, i);
      } else c & 16 ? cm(e, t, r, n, o, i) : Kn(t, e, o, a, i);
    n = s ? n.projectionNext : n.next;
  }
}
function Ks(e, t, n, r, o, i) {
  Hl(n, r, e.firstChild, t, o, i, !1);
}
function oC(e, t, n) {
  let r = t[U],
    o = tm(e, n, t),
    i = n.parent || t[De],
    s = rm(i, n, t);
  cm(r, 0, t, n, o, s);
}
function cm(e, t, n, r, o, i) {
  let s = n[be],
    c = s[De].projection[r.projection];
  if (Array.isArray(c))
    for (let u = 0; u < c.length; u++) {
      let l = c[u];
      Kn(t, e, o, l, i);
    }
  else {
    let u = c,
      l = s[he];
    (mg(r) && (u.flags |= 128), Hl(e, t, u, l, o, i, !0));
  }
}
function iC(e, t, n, r, o) {
  let i = n[pn],
    s = Pe(n);
  i !== s && Kn(t, e, r, i, o);
  for (let a = fe; a < n.length; a++) {
    let c = n[a];
    Ks(c[N], c, e, t, r, i);
  }
}
function sC(e, t, n, r, o) {
  if (t) o ? e.addClass(n, r) : e.removeClass(n, r);
  else {
    let i = r.indexOf("-") === -1 ? void 0 : Je.DashCase;
    o == null
      ? e.removeStyle(n, r, i)
      : (typeof o == "string" &&
          o.endsWith("!important") &&
          ((o = o.slice(0, -10)), (i |= Je.Important)),
        e.setStyle(n, r, o, i));
  }
}
function vs(e, t, n, r, o = !1) {
  for (; n !== null; ) {
    if (n.type === 128) {
      n = o ? n.projectionNext : n.next;
      continue;
    }
    let i = t[n.index];
    (i !== null && r.push(Pe(i)), wt(i) && aC(i, r));
    let s = n.type;
    if (s & 8) vs(e, t, n.child, r);
    else if (s & 32) {
      let a = Bl(n, t),
        c;
      for (; (c = a()); ) r.push(c);
    } else if (s & 16) {
      let a = am(t, n);
      if (Array.isArray(a)) r.push(...a);
      else {
        let c = gn(t[be]);
        vs(c[N], c, a, r, !0);
      }
    }
    n = o ? n.projectionNext : n.next;
  }
  return r;
}
function aC(e, t) {
  for (let n = fe; n < e.length; n++) {
    let r = e[n],
      o = r[N].firstChild;
    o !== null && vs(r[N], r, o, t);
  }
  e[pn] !== e[Dt] && t.push(e[pn]);
}
function um(e) {
  if (e[Jn] !== null) {
    for (let t of e[Jn]) t.impl.addSequence(t);
    e[Jn].length = 0;
  }
}
var lm = [];
function cC(e) {
  return e[ke] ?? uC(e);
}
function uC(e) {
  let t = lm.pop() ?? Object.create(dC);
  return ((t.lView = e), t);
}
function lC(e) {
  e.lView[ke] !== e && ((e.lView = null), lm.push(e));
}
var dC = j(D({}, Xt), {
  consumerIsAlwaysLive: !0,
  kind: "template",
  consumerMarkedDirty: (e) => {
    Dr(e.lView);
  },
  consumerOnSignalRead() {
    this.lView[ke] = this;
  },
});
function fC(e) {
  let t = e[ke] ?? Object.create(hC);
  return ((t.lView = e), t);
}
var hC = j(D({}, Xt), {
  consumerIsAlwaysLive: !0,
  kind: "template",
  consumerMarkedDirty: (e) => {
    let t = gn(e.lView);
    for (; t && !dm(t[N]); ) t = gn(t);
    t && Lp(t);
  },
  consumerOnSignalRead() {
    this.lView[ke] = this;
  },
});
function dm(e) {
  return e.type !== 2;
}
function fm(e) {
  if (e[hn] === null) return;
  let t = !0;
  for (; t; ) {
    let n = !1;
    for (let r of e[hn])
      r.dirty &&
        ((n = !0),
        r.zone === null || Zone.current === r.zone
          ? r.run()
          : r.zone.run(() => r.run()));
    t = n && !!(e[_] & 8192);
  }
}
var pC = 100;
function hm(e, t = !0, n = 0) {
  let o = e[ht].rendererFactory,
    i = !1;
  i || o.begin?.();
  try {
    gC(e, n);
  } catch (s) {
    throw (t && ZI(e, s), s);
  } finally {
    i || o.end?.();
  }
}
function gC(e, t) {
  let n = Hp();
  try {
    (us(!0), hu(e, t));
    let r = 0;
    for (; Vs(e); ) {
      if (r === pC) throw new v(103, !1);
      (r++, hu(e, 1));
    }
  } finally {
    us(n);
  }
}
function mC(e, t, n, r) {
  if (Eo(t)) return;
  let o = t[_],
    i = !1,
    s = !1;
  ll(t);
  let a = !0,
    c = null,
    u = null;
  i ||
    (dm(e)
      ? ((u = cC(t)), (c = en(u)))
      : Ha() === null
        ? ((a = !1), (u = fC(t)), (c = en(u)))
        : t[ke] && (Gr(t[ke]), (t[ke] = null)));
  try {
    (Fp(t), oE(e.bindingStartIndex), n !== null && Xg(e, t, n, 2, r));
    let l = (o & 3) === 3;
    if (!i)
      if (l) {
        let f = e.preOrderCheckHooks;
        f !== null && Zi(t, f, null);
      } else {
        let f = e.preOrderHooks;
        (f !== null && Yi(t, f, 0, null), yc(t, 0));
      }
    if (
      (s || yC(t), fm(t), pm(t, 0), e.contentQueries !== null && _g(e, t), !i)
    )
      if (l) {
        let f = e.contentCheckHooks;
        f !== null && Zi(t, f);
      } else {
        let f = e.contentHooks;
        (f !== null && Yi(t, f, 1), yc(t, 1));
      }
    DC(e, t);
    let d = e.components;
    d !== null && mm(t, d, 0);
    let h = e.viewQuery;
    if ((h !== null && Jc(2, h, r), !i))
      if (l) {
        let f = e.viewCheckHooks;
        f !== null && Zi(t, f);
      } else {
        let f = e.viewHooks;
        (f !== null && Yi(t, f, 2), yc(t, 2));
      }
    if ((e.firstUpdatePass === !0 && (e.firstUpdatePass = !1), t[mc])) {
      for (let f of t[mc]) f();
      t[mc] = null;
    }
    i || (um(t), (t[_] &= -73));
  } catch (l) {
    throw (i || Dr(t), l);
  } finally {
    (u !== null && (kn(u, c), a && lC(u)), dl());
  }
}
function pm(e, t) {
  for (let n = Dg(e); n !== null; n = wg(n))
    for (let r = fe; r < n.length; r++) {
      let o = n[r];
      gm(o, t);
    }
}
function yC(e) {
  for (let t = Dg(e); t !== null; t = wg(t)) {
    if (!(t[_] & 2)) continue;
    let n = t[or];
    for (let r = 0; r < n.length; r++) {
      let o = n[r];
      Lp(o);
    }
  }
}
function vC(e, t, n) {
  $(18);
  let r = Ye(t, e);
  (gm(r, n), $(19, r[ne]));
}
function gm(e, t) {
  ol(e) && hu(e, t);
}
function hu(e, t) {
  let r = e[N],
    o = e[_],
    i = e[ke],
    s = !!(t === 0 && o & 16);
  if (
    ((s ||= !!(o & 64 && t === 0)),
    (s ||= !!(o & 1024)),
    (s ||= !!(i?.dirty && qr(i))),
    (s ||= !1),
    i && (i.dirty = !1),
    (e[_] &= -9217),
    s)
  )
    mC(r, e, r.template, e[ne]);
  else if (o & 8192) {
    (fm(e), pm(e, 1));
    let a = r.components;
    (a !== null && mm(e, a, 1), um(e));
  }
}
function mm(e, t, n) {
  for (let r = 0; r < t.length; r++) vC(e, t[r], n);
}
function DC(e, t) {
  let n = e.hostBindingOpCodes;
  if (n !== null)
    try {
      for (let r = 0; r < n.length; r++) {
        let o = n[r];
        if (o < 0) mn(~o);
        else {
          let i = o,
            s = n[++r],
            a = n[++r];
          sE(s, i);
          let c = t[i];
          ($(24, c), a(2, c), $(25, c));
        }
      }
    } finally {
      mn(-1);
    }
}
function zl(e, t) {
  let n = Hp() ? 64 : 1088;
  for (e[ht].changeDetectionScheduler?.notify(t); e; ) {
    e[_] |= n;
    let r = gn(e);
    if (cs(e) && !r) return e;
    e = r;
  }
  return null;
}
function ym(e, t, n, r) {
  return [e, !0, 0, t, null, r, null, n, null, null];
}
function vm(e, t) {
  let n = fe + t;
  if (n < e.length) return e[n];
}
function xo(e, t, n, r = !0) {
  let o = t[N];
  if ((wC(o, t, e, n), r)) {
    let s = fu(n, e),
      a = t[U],
      c = a.parentNode(e[pn]);
    c !== null && eC(o, e[De], a, t, c, s);
  }
  let i = t[nr];
  i !== null && i.firstChild !== null && (i.firstChild = null);
}
function Dm(e, t) {
  let n = fo(e, t);
  return (n !== void 0 && Ys(n[N], n), n);
}
function fo(e, t) {
  if (e.length <= fe) return;
  let n = fe + t,
    r = e[n];
  if (r) {
    let o = r[fn];
    (o !== null && o !== e && $l(o, r), t > 0 && (e[n - 1][Be] = r[Be]));
    let i = os(e, fe + t);
    XI(r[N], r);
    let s = i[pt];
    (s !== null && s.detachView(i[N]),
      (r[he] = null),
      (r[Be] = null),
      (r[_] &= -129));
  }
  return r;
}
function wC(e, t, n, r) {
  let o = fe + r,
    i = n.length;
  (r > 0 && (n[o - 1][Be] = t),
    r < i - fe
      ? ((t[Be] = n[o]), Dp(n, fe + r, t))
      : (n.push(t), (t[Be] = null)),
    (t[he] = n));
  let s = t[fn];
  s !== null && n !== s && wm(s, t);
  let a = t[pt];
  (a !== null && a.insertView(e), Vc(t), (t[_] |= 128));
}
function wm(e, t) {
  let n = e[or],
    r = t[he];
  if (Bt(r)) e[_] |= 2;
  else {
    let o = r[he][be];
    t[be] !== o && (e[_] |= 2);
  }
  n === null ? (e[or] = [t]) : n.push(t);
}
var ho = class {
  _lView;
  _cdRefInjectingView;
  notifyErrorHandler;
  _appRef = null;
  _attachedToViewContainer = !1;
  get rootNodes() {
    let t = this._lView,
      n = t[N];
    return vs(n, t, n.firstChild, []);
  }
  constructor(t, n, r = !0) {
    ((this._lView = t),
      (this._cdRefInjectingView = n),
      (this.notifyErrorHandler = r));
  }
  get context() {
    return this._lView[ne];
  }
  set context(t) {
    this._lView[ne] = t;
  }
  get destroyed() {
    return Eo(this._lView);
  }
  destroy() {
    if (this._appRef) this._appRef.detachView(this);
    else if (this._attachedToViewContainer) {
      let t = this._lView[he];
      if (wt(t)) {
        let n = t[as],
          r = n ? n.indexOf(this) : -1;
        r > -1 && (fo(t, r), os(n, r));
      }
      this._attachedToViewContainer = !1;
    }
    Ys(this._lView[N], this._lView);
  }
  onDestroy(t) {
    jp(this._lView, t);
  }
  markForCheck() {
    zl(this._cdRefInjectingView || this._lView, 4);
  }
  detach() {
    this._lView[_] &= -129;
  }
  reattach() {
    (Vc(this._lView), (this._lView[_] |= 128));
  }
  detectChanges() {
    ((this._lView[_] |= 1024), hm(this._lView, this.notifyErrorHandler));
  }
  checkNoChanges() {}
  attachToViewContainerRef() {
    if (this._appRef) throw new v(902, !1);
    this._attachedToViewContainer = !0;
  }
  detachFromAppRef() {
    this._appRef = null;
    let t = cs(this._lView),
      n = this._lView[fn];
    (n !== null && !t && $l(n, this._lView), em(this._lView[N], this._lView));
  }
  attachToAppRef(t) {
    if (this._attachedToViewContainer) throw new v(902, !1);
    this._appRef = t;
    let n = cs(this._lView),
      r = this._lView[fn];
    (r !== null && !n && wm(r, this._lView), Vc(this._lView));
  }
};
var Dn = (() => {
    class e {
      static __NG_ELEMENT_ID__ = CC;
    }
    return e;
  })(),
  EC = Dn,
  IC = class extends EC {
    _declarationLView;
    _declarationTContainer;
    elementRef;
    constructor(t, n, r) {
      (super(),
        (this._declarationLView = t),
        (this._declarationTContainer = n),
        (this.elementRef = r));
    }
    get ssrId() {
      return this._declarationTContainer.tView?.ssrId || null;
    }
    createEmbeddedView(t, n) {
      return this.createEmbeddedViewImpl(t, n);
    }
    createEmbeddedViewImpl(t, n, r) {
      let o = _o(this._declarationLView, this._declarationTContainer, t, {
        embeddedViewInjector: n,
        dehydratedView: r,
      });
      return new ho(o);
    }
  };
function CC() {
  return Js(we(), C());
}
function Js(e, t) {
  return e.type & 4 ? new IC(t, e, wr(e, t)) : null;
}
function Em(e, t, n) {
  let r = t.insertBeforeIndex,
    o = Array.isArray(r) ? r[0] : r;
  return o === null ? om(e, t, n) : Pe(n[o]);
}
function Im(e, t, n, r, o) {
  let i = t.insertBeforeIndex;
  if (Array.isArray(i)) {
    let s = r,
      a = null;
    if (
      (t.type & 3 || ((a = s), (s = o)), s !== null && t.componentOffset === -1)
    )
      for (let c = 1; c < i.length; c++) {
        let u = n[i[c]];
        vn(e, s, u, a, !1);
      }
  }
}
function No(e, t, n, r, o) {
  let i = e.data[t];
  if (i === null) ((i = ql(e, t, n, r, o)), iE() && (i.flags |= 32));
  else if (i.type & 64) {
    ((i.type = n), (i.value = r), (i.attrs = o));
    let s = co();
    i.injectorIndex = s === null ? -1 : s.injectorIndex;
  }
  return (Qe(i, !0), i);
}
function ql(e, t, n, r, o) {
  let i = $p(),
    s = sl(),
    a = s ? i : i && i.parent,
    c = (e.data[t] = MC(e, a, n, t, r, o));
  return (bC(e, c, i, s), c);
}
function bC(e, t, n, r) {
  (e.firstChild === null && (e.firstChild = t),
    n !== null &&
      (r
        ? n.child == null && t.parent !== null && (n.child = t)
        : n.next === null && ((n.next = t), (t.prev = n))));
}
function MC(e, t, n, r, o, i) {
  let s = t ? t.injectorIndex : -1,
    a = 0;
  return (
    Up() && (a |= 128),
    {
      type: n,
      index: r,
      insertBeforeIndex: null,
      injectorIndex: s,
      directiveStart: -1,
      directiveEnd: -1,
      directiveStylingLast: -1,
      componentOffset: -1,
      propertyBindings: null,
      flags: a,
      providerIndexes: 0,
      value: o,
      attrs: i,
      mergedAttrs: null,
      localNames: null,
      initialInputs: null,
      inputs: null,
      hostDirectiveInputs: null,
      outputs: null,
      hostDirectiveOutputs: null,
      directiveToIndex: null,
      tView: null,
      next: null,
      prev: null,
      projectionNext: null,
      child: null,
      parent: t,
      projection: null,
      styles: null,
      stylesWithoutHost: null,
      residualStyles: void 0,
      classes: null,
      classesWithoutHost: null,
      residualClasses: void 0,
      classBindings: 0,
      styleBindings: 0,
    }
  );
}
function Cm(e, t) {
  if ((e.push(t), e.length > 1))
    for (let n = e.length - 2; n >= 0; n--) {
      let r = e[n];
      bm(r) || (SC(r, t) && TC(r) === null && _C(r, t.index));
    }
}
function bm(e) {
  return !(e.type & 64);
}
function SC(e, t) {
  return bm(t) || e.index > t.index;
}
function TC(e) {
  let t = e.insertBeforeIndex;
  return Array.isArray(t) ? t[0] : t;
}
function _C(e, t) {
  let n = e.insertBeforeIndex;
  Array.isArray(n) ? (n[0] = t) : (sm(Em, Im), (e.insertBeforeIndex = t));
}
function no(e, t) {
  let n = e.data[t];
  return n === null || typeof n == "string"
    ? null
    : n.hasOwnProperty("currentCaseLViewIndex")
      ? n
      : n.value;
}
function xC(e, t, n) {
  let r = e.data[t];
  r === null ? (e.data[t] = n) : (r.value = n);
}
function NC(e, t) {
  let n = e.insertBeforeIndex;
  n === null
    ? (sm(Em, Im), (n = e.insertBeforeIndex = [null, t]))
    : (uw(Array.isArray(n), !0, "Expecting array here"), n.push(t));
}
function RC(e, t, n) {
  let r = ql(e, n, 64, null, null);
  return (Cm(t, r), r);
}
function Xs(e, t) {
  let n = t[e.currentCaseLViewIndex];
  return n === null ? n : n < 0 ? ~n : n;
}
function AC(e) {
  return e >>> 17;
}
function OC(e) {
  return (e & 131070) >>> 1;
}
function kC(e, t, n) {
  return e | (t << 17) | (n << 1);
}
function PC(e) {
  return e === -1;
}
function Mm(e, t, n) {
  e.index = 0;
  let r = Xs(t, n);
  r !== null ? (e.removes = t.remove[r]) : (e.removes = de);
}
function pu(e) {
  if (e.index < e.removes.length) {
    let t = e.removes[e.index++];
    if (t > 0) return e.lView[t];
    {
      e.stack.push(e.index, e.removes);
      let n = ~t,
        r = e.lView[N].data[n];
      return (Mm(e, r, e.lView), pu(e));
    }
  } else
    return e.stack.length === 0
      ? null
      : ((e.removes = e.stack.pop()), (e.index = e.stack.pop()), pu(e));
}
function FC() {
  let e = { stack: [], index: -1 };
  function t(n, r) {
    for (e.lView = r; e.stack.length; ) e.stack.pop();
    return (Mm(e, n.value, r), pu.bind(null, e));
  }
  return t;
}
var g1 = new RegExp(`^(\\d+)*(${GE}|${qE})*(.*)`);
var LC = () => {};
function jC(e, t, n, r) {
  LC(e, t, n, r);
}
var VC = () => {};
function BC(e, t, n) {
  VC(e, t, n);
}
var UC = () => null;
function cr(e, t) {
  return UC(e, t);
}
var $C = class {},
  Sm = class {},
  gu = class {
    resolveComponentFactory(t) {
      throw Error(`No component factory found for ${Ce(t)}.`);
    }
  },
  ea = class {
    static NULL = new gu();
  },
  ur = class {},
  Er = (() => {
    class e {
      destroyNode = null;
      static __NG_ELEMENT_ID__ = () => HC();
    }
    return e;
  })();
function HC() {
  let e = C(),
    t = we(),
    n = Ye(t.index, e);
  return (Bt(n) ? n : e)[U];
}
var zC = (() => {
  class e {
    static ɵprov = E({ token: e, providedIn: "root", factory: () => null });
  }
  return e;
})();
function mu(e, t, n) {
  let r = n ? e.styles : null,
    o = n ? e.classes : null,
    i = 0;
  if (t !== null)
    for (let s = 0; s < t.length; s++) {
      let a = t[s];
      if (typeof a == "number") i = a;
      else if (i == 1) o = Rc(o, a);
      else if (i == 2) {
        let c = a,
          u = t[++s];
        r = Rc(r, c + ": " + u + ";");
      }
    }
  (n ? (e.styles = r) : (e.stylesWithoutHost = r),
    n ? (e.classes = o) : (e.classesWithoutHost = o));
}
function q(e, t = P.Default) {
  let n = C();
  if (n === null) return b(e, t);
  let r = we();
  return sg(r, n, ae(e), t);
}
function Tm() {
  let e = "invalid";
  throw new Error(e);
}
function Gl(e, t, n, r, o) {
  let i = r === null ? null : { "": -1 },
    s = o(e, n);
  if (s !== null) {
    let a,
      c = null,
      u = null,
      l = GC(s);
    (l === null ? (a = s) : ([a, c, u] = l), YC(e, t, n, a, i, c, u));
  }
  i !== null && r !== null && qC(n, r, i);
}
function qC(e, t, n) {
  let r = (e.localNames = []);
  for (let o = 0; o < t.length; o += 2) {
    let i = n[t[o + 1]];
    if (i == null) throw new v(-301, !1);
    r.push(t[o], i);
  }
}
function GC(e) {
  let t = null,
    n = !1;
  for (let s = 0; s < e.length; s++) {
    let a = e[s];
    if ((s === 0 && Ze(a) && (t = a), a.findHostDirectiveDefs !== null)) {
      n = !0;
      break;
    }
  }
  if (!n) return null;
  let r = null,
    o = null,
    i = null;
  for (let s of e)
    (s.findHostDirectiveDefs !== null &&
      ((r ??= []), (o ??= new Map()), (i ??= new Map()), WC(s, r, i, o)),
      s === t && ((r ??= []), r.push(s)));
  return r !== null
    ? (r.push(...(t === null ? e : e.slice(1))), [r, o, i])
    : null;
}
function WC(e, t, n, r) {
  let o = t.length;
  (e.findHostDirectiveDefs(e, t, r), n.set(e, [o, t.length - 1]));
}
function ZC(e, t, n) {
  ((t.componentOffset = n), (e.components ??= []).push(t.index));
}
function YC(e, t, n, r, o, i, s) {
  let a = r.length,
    c = !1;
  for (let h = 0; h < a; h++) {
    let f = r[h];
    (!c && Ze(f) && ((c = !0), ZC(e, n, h)), zc(hs(n, t), e, f.type));
  }
  tb(n, e.data.length, a);
  for (let h = 0; h < a; h++) {
    let f = r[h];
    f.providersResolver && f.providersResolver(f);
  }
  let u = !1,
    l = !1,
    d = To(e, t, a, null);
  a > 0 && (n.directiveToIndex = new Map());
  for (let h = 0; h < a; h++) {
    let f = r[h];
    if (
      ((n.mergedAttrs = ir(n.mergedAttrs, f.hostAttrs)),
      KC(e, n, t, d, f),
      eb(d, f, o),
      s !== null && s.has(f))
    ) {
      let [g, y] = s.get(f);
      n.directiveToIndex.set(f.type, [
        d,
        g + n.directiveStart,
        y + n.directiveStart,
      ]);
    } else (i === null || !i.has(f)) && n.directiveToIndex.set(f.type, d);
    (f.contentQueries !== null && (n.flags |= 4),
      (f.hostBindings !== null || f.hostAttrs !== null || f.hostVars !== 0) &&
        (n.flags |= 64));
    let p = f.type.prototype;
    (!u &&
      (p.ngOnChanges || p.ngOnInit || p.ngDoCheck) &&
      ((e.preOrderHooks ??= []).push(n.index), (u = !0)),
      !l &&
        (p.ngOnChanges || p.ngDoCheck) &&
        ((e.preOrderCheckHooks ??= []).push(n.index), (l = !0)),
      d++);
  }
  QC(e, n, i);
}
function QC(e, t, n) {
  for (let r = t.directiveStart; r < t.directiveEnd; r++) {
    let o = e.data[r];
    if (n === null || !n.has(o)) (_h(0, t, o, r), _h(1, t, o, r), Nh(t, r, !1));
    else {
      let i = n.get(o);
      (xh(0, t, i, r), xh(1, t, i, r), Nh(t, r, !0));
    }
  }
}
function _h(e, t, n, r) {
  let o = e === 0 ? n.inputs : n.outputs;
  for (let i in o)
    if (o.hasOwnProperty(i)) {
      let s;
      (e === 0 ? (s = t.inputs ??= {}) : (s = t.outputs ??= {}),
        (s[i] ??= []),
        s[i].push(r),
        _m(t, i));
    }
}
function xh(e, t, n, r) {
  let o = e === 0 ? n.inputs : n.outputs;
  for (let i in o)
    if (o.hasOwnProperty(i)) {
      let s = o[i],
        a;
      (e === 0
        ? (a = t.hostDirectiveInputs ??= {})
        : (a = t.hostDirectiveOutputs ??= {}),
        (a[s] ??= []),
        a[s].push(r, i),
        _m(t, s));
    }
}
function _m(e, t) {
  t === "class" ? (e.flags |= 8) : t === "style" && (e.flags |= 16);
}
function Nh(e, t, n) {
  let { attrs: r, inputs: o, hostDirectiveInputs: i } = e;
  if (r === null || (!n && o === null) || (n && i === null) || xl(e)) {
    ((e.initialInputs ??= []), e.initialInputs.push(null));
    return;
  }
  let s = null,
    a = 0;
  for (; a < r.length; ) {
    let c = r[a];
    if (c === 0) {
      a += 4;
      continue;
    } else if (c === 5) {
      a += 2;
      continue;
    } else if (typeof c == "number") break;
    if (!n && o.hasOwnProperty(c)) {
      let u = o[c];
      for (let l of u)
        if (l === t) {
          ((s ??= []), s.push(c, r[a + 1]));
          break;
        }
    } else if (n && i.hasOwnProperty(c)) {
      let u = i[c];
      for (let l = 0; l < u.length; l += 2)
        if (u[l] === t) {
          ((s ??= []), s.push(u[l + 1], r[a + 1]));
          break;
        }
    }
    a += 2;
  }
  ((e.initialInputs ??= []), e.initialInputs.push(s));
}
function KC(e, t, n, r, o) {
  e.data[r] = o;
  let i = o.factory || (o.factory = dn(o.type, !0)),
    s = new yn(i, Ze(o), q);
  ((e.blueprint[r] = s), (n[r] = s), JC(e, t, r, To(e, n, o.hostVars, Se), o));
}
function JC(e, t, n, r, o) {
  let i = o.hostBindings;
  if (i) {
    let s = e.hostBindingOpCodes;
    s === null && (s = e.hostBindingOpCodes = []);
    let a = ~t.index;
    (XC(s) != a && s.push(a), s.push(n, r, i));
  }
}
function XC(e) {
  let t = e.length;
  for (; t > 0; ) {
    let n = e[--t];
    if (typeof n == "number" && n < 0) return n;
  }
  return 0;
}
function eb(e, t, n) {
  if (n) {
    if (t.exportAs)
      for (let r = 0; r < t.exportAs.length; r++) n[t.exportAs[r]] = e;
    Ze(t) && (n[""] = e);
  }
}
function tb(e, t, n) {
  ((e.flags |= 1),
    (e.directiveStart = t),
    (e.directiveEnd = t + n),
    (e.providerIndexes = t));
}
function xm(e, t, n, r, o, i, s, a) {
  let c = t.consts,
    u = gt(c, s),
    l = No(t, e, 2, r, u);
  return (
    i && Gl(t, n, l, gt(c, a), o),
    (l.mergedAttrs = ir(l.mergedAttrs, l.attrs)),
    l.attrs !== null && mu(l, l.attrs, !1),
    l.mergedAttrs !== null && mu(l, l.mergedAttrs, !0),
    t.queries !== null && t.queries.elementStart(t, l),
    l
  );
}
function Nm(e, t) {
  (fl(e, t), nl(t) && e.queries.elementEnd(t));
}
var Ds = class extends ea {
  ngModule;
  constructor(t) {
    (super(), (this.ngModule = t));
  }
  resolveComponentFactory(t) {
    let n = $t(t);
    return new wn(n, this.ngModule);
  }
};
function nb(e) {
  return Object.keys(e).map((t) => {
    let [n, r, o] = e[t],
      i = {
        propName: n,
        templateName: t,
        isSignal: (r & Gs.SignalBased) !== 0,
      };
    return (o && (i.transform = o), i);
  });
}
function rb(e) {
  return Object.keys(e).map((t) => ({ propName: e[t], templateName: t }));
}
function ob(e, t, n) {
  let r = t instanceof ye ? t : t?.injector;
  return (
    r &&
      e.getStandaloneInjector !== null &&
      (r = e.getStandaloneInjector(r) || r),
    r ? new $c(n, r) : n
  );
}
function ib(e) {
  let t = e.get(ur, null);
  if (t === null) throw new v(407, !1);
  let n = e.get(zC, null),
    r = e.get(mt, null);
  return { rendererFactory: t, sanitizer: n, changeDetectionScheduler: r };
}
function sb(e, t) {
  let n = (e.selectors[0][0] || "div").toLowerCase();
  return Nl(t, n, n === "svg" ? Pp : n === "math" ? Gw : null);
}
var wn = class extends Sm {
    componentDef;
    ngModule;
    selector;
    componentType;
    ngContentSelectors;
    isBoundToModule;
    cachedInputs = null;
    cachedOutputs = null;
    get inputs() {
      return (
        (this.cachedInputs ??= nb(this.componentDef.inputs)),
        this.cachedInputs
      );
    }
    get outputs() {
      return (
        (this.cachedOutputs ??= rb(this.componentDef.outputs)),
        this.cachedOutputs
      );
    }
    constructor(t, n) {
      (super(),
        (this.componentDef = t),
        (this.ngModule = n),
        (this.componentType = t.type),
        (this.selector = RI(t.selectors)),
        (this.ngContentSelectors = t.ngContentSelectors ?? []),
        (this.isBoundToModule = !!n));
    }
    create(t, n, r, o) {
      $(22);
      let i = A(null);
      try {
        let s = this.componentDef,
          a = r ? ["ng-version", "19.2.4"] : AI(this.componentDef.selectors[0]),
          c = Rl(0, null, null, 1, 0, null, null, null, null, [a], null),
          u = ob(s, o || this.ngModule, t),
          l = ib(u),
          d = l.rendererFactory.createRenderer(null, s),
          h = r ? jI(d, r, s.encapsulation, u) : sb(s, d),
          f = Al(
            null,
            c,
            null,
            512 | Kg(s),
            null,
            null,
            l,
            d,
            u,
            null,
            Tg(h, u, !0),
          );
        ((f[Y] = h), ll(f));
        let p = null;
        try {
          let g = xm(Y, c, f, "#host", () => [this.componentDef], !0, 0);
          (h && (Qg(d, h, g), Ht(h, f)),
            Ws(c, f, g),
            Ml(c, g, f),
            Nm(c, g),
            n !== void 0 && ab(g, this.ngContentSelectors, n),
            (p = Ye(g.index, f)),
            (f[ne] = p[ne]),
            Vl(c, f, null));
        } catch (g) {
          throw (p !== null && Qc(p), Qc(f), g);
        } finally {
          ($(23), dl());
        }
        return new yu(this.componentType, f);
      } finally {
        A(i);
      }
    }
  },
  yu = class extends $C {
    _rootLView;
    instance;
    hostView;
    changeDetectorRef;
    componentType;
    location;
    previousInputValues = null;
    _tNode;
    constructor(t, n) {
      (super(),
        (this._rootLView = n),
        (this._tNode = rl(n[N], Y)),
        (this.location = wr(this._tNode, n)),
        (this.instance = Ye(this._tNode.index, n)[ne]),
        (this.hostView = this.changeDetectorRef = new ho(n, void 0, !1)),
        (this.componentType = t));
    }
    setInput(t, n) {
      let r = this._tNode;
      if (
        ((this.previousInputValues ??= new Map()),
        this.previousInputValues.has(t) &&
          Object.is(this.previousInputValues.get(t), n))
      )
        return;
      let o = this._rootLView,
        i = jl(r, o[N], o, t, n);
      this.previousInputValues.set(t, n);
      let s = Ye(r.index, o);
      zl(s, 1);
    }
    get injector() {
      return new ln(this._tNode, this._rootLView);
    }
    destroy() {
      this.hostView.destroy();
    }
    onDestroy(t) {
      this.hostView.onDestroy(t);
    }
  };
function ab(e, t, n) {
  let r = (e.projection = []);
  for (let o = 0; o < t.length; o++) {
    let i = n[o];
    r.push(i != null && i.length ? Array.from(i) : null);
  }
}
var nt = (() => {
  class e {
    static __NG_ELEMENT_ID__ = cb;
  }
  return e;
})();
function cb() {
  let e = we();
  return Am(e, C());
}
var ub = nt,
  Rm = class extends ub {
    _lContainer;
    _hostTNode;
    _hostLView;
    constructor(t, n, r) {
      (super(),
        (this._lContainer = t),
        (this._hostTNode = n),
        (this._hostLView = r));
    }
    get element() {
      return wr(this._hostTNode, this._hostLView);
    }
    get injector() {
      return new ln(this._hostTNode, this._hostLView);
    }
    get parentInjector() {
      let t = hl(this._hostTNode, this._hostLView);
      if (eg(t)) {
        let n = ds(t, this._hostLView),
          r = ls(t),
          o = n[N].data[r + 8];
        return new ln(o, n);
      } else return new ln(null, this._hostLView);
    }
    clear() {
      for (; this.length > 0; ) this.remove(this.length - 1);
    }
    get(t) {
      let n = Rh(this._lContainer);
      return (n !== null && n[t]) || null;
    }
    get length() {
      return this._lContainer.length - fe;
    }
    createEmbeddedView(t, n, r) {
      let o, i;
      typeof r == "number"
        ? (o = r)
        : r != null && ((o = r.index), (i = r.injector));
      let s = cr(this._lContainer, t.ssrId),
        a = t.createEmbeddedViewImpl(n || {}, i, s);
      return (this.insertImpl(a, o, ar(this._hostTNode, s)), a);
    }
    createComponent(t, n, r, o, i) {
      let s = t && !$w(t),
        a;
      if (s) a = n;
      else {
        let p = n || {};
        ((a = p.index),
          (r = p.injector),
          (o = p.projectableNodes),
          (i = p.environmentInjector || p.ngModuleRef));
      }
      let c = s ? t : new wn($t(t)),
        u = r || this.parentInjector;
      if (!i && c.ngModule == null) {
        let g = (s ? u : this.parentInjector).get(ye, null);
        g && (i = g);
      }
      let l = $t(c.componentType ?? {}),
        d = cr(this._lContainer, l?.id ?? null),
        h = d?.firstChild ?? null,
        f = c.create(u, o, h, i);
      return (this.insertImpl(f.hostView, a, ar(this._hostTNode, d)), f);
    }
    insert(t, n) {
      return this.insertImpl(t, n, !0);
    }
    insertImpl(t, n, r) {
      let o = t._lView;
      if (Zw(o)) {
        let a = this.indexOf(t);
        if (a !== -1) this.detach(a);
        else {
          let c = o[he],
            u = new Rm(c, c[De], c[he]);
          u.detach(u.indexOf(t));
        }
      }
      let i = this._adjustIndex(n),
        s = this._lContainer;
      return (xo(s, o, i, r), t.attachToViewContainerRef(), Dp(Ec(s), i, t), t);
    }
    move(t, n) {
      return this.insert(t, n);
    }
    indexOf(t) {
      let n = Rh(this._lContainer);
      return n !== null ? n.indexOf(t) : -1;
    }
    remove(t) {
      let n = this._adjustIndex(t, -1),
        r = fo(this._lContainer, n);
      r && (os(Ec(this._lContainer), n), Ys(r[N], r));
    }
    detach(t) {
      let n = this._adjustIndex(t, -1),
        r = fo(this._lContainer, n);
      return r && os(Ec(this._lContainer), n) != null ? new ho(r) : null;
    }
    _adjustIndex(t, n = 0) {
      return t ?? this.length + n;
    }
  };
function Rh(e) {
  return e[as];
}
function Ec(e) {
  return e[as] || (e[as] = []);
}
function Am(e, t) {
  let n,
    r = t[e.index];
  return (
    wt(r) ? (n = r) : ((n = ym(r, t, null, e)), (t[e.index] = n), Ol(t, n)),
    db(n, t, e, r),
    new Rm(n, e, t)
  );
}
function lb(e, t) {
  let n = e[U],
    r = n.createComment(""),
    o = Xe(t, e),
    i = n.parentNode(o);
  return (vn(n, i, r, n.nextSibling(o), !1), r);
}
var db = pb,
  fb = () => !1;
function hb(e, t, n) {
  return fb(e, t, n);
}
function pb(e, t, n, r) {
  if (e[pn]) return;
  let o;
  (n.type & 8 ? (o = Pe(r)) : (o = lb(t, n)), (e[pn] = o));
}
var vu = class e {
    queryList;
    matches = null;
    constructor(t) {
      this.queryList = t;
    }
    clone() {
      return new e(this.queryList);
    }
    setDirty() {
      this.queryList.setDirty();
    }
  },
  Du = class e {
    queries;
    constructor(t = []) {
      this.queries = t;
    }
    createEmbeddedView(t) {
      let n = t.queries;
      if (n !== null) {
        let r = t.contentQueries !== null ? t.contentQueries[0] : n.length,
          o = [];
        for (let i = 0; i < r; i++) {
          let s = n.getByIndex(i),
            a = this.queries[s.indexInDeclarationView];
          o.push(a.clone());
        }
        return new e(o);
      }
      return null;
    }
    insertView(t) {
      this.dirtyQueriesWithMatches(t);
    }
    detachView(t) {
      this.dirtyQueriesWithMatches(t);
    }
    finishViewCreation(t) {
      this.dirtyQueriesWithMatches(t);
    }
    dirtyQueriesWithMatches(t) {
      for (let n = 0; n < this.queries.length; n++)
        Zl(t, n).matches !== null && this.queries[n].setDirty();
    }
  },
  ws = class {
    flags;
    read;
    predicate;
    constructor(t, n, r = null) {
      ((this.flags = n),
        (this.read = r),
        typeof t == "string" ? (this.predicate = wb(t)) : (this.predicate = t));
    }
  },
  wu = class e {
    queries;
    constructor(t = []) {
      this.queries = t;
    }
    elementStart(t, n) {
      for (let r = 0; r < this.queries.length; r++)
        this.queries[r].elementStart(t, n);
    }
    elementEnd(t) {
      for (let n = 0; n < this.queries.length; n++)
        this.queries[n].elementEnd(t);
    }
    embeddedTView(t) {
      let n = null;
      for (let r = 0; r < this.length; r++) {
        let o = n !== null ? n.length : 0,
          i = this.getByIndex(r).embeddedTView(t, o);
        i &&
          ((i.indexInDeclarationView = r), n !== null ? n.push(i) : (n = [i]));
      }
      return n !== null ? new e(n) : null;
    }
    template(t, n) {
      for (let r = 0; r < this.queries.length; r++)
        this.queries[r].template(t, n);
    }
    getByIndex(t) {
      return this.queries[t];
    }
    get length() {
      return this.queries.length;
    }
    track(t) {
      this.queries.push(t);
    }
  },
  Eu = class e {
    metadata;
    matches = null;
    indexInDeclarationView = -1;
    crossesNgTemplate = !1;
    _declarationNodeIndex;
    _appliesToNextNode = !0;
    constructor(t, n = -1) {
      ((this.metadata = t), (this._declarationNodeIndex = n));
    }
    elementStart(t, n) {
      this.isApplyingToNode(n) && this.matchTNode(t, n);
    }
    elementEnd(t) {
      this._declarationNodeIndex === t.index && (this._appliesToNextNode = !1);
    }
    template(t, n) {
      this.elementStart(t, n);
    }
    embeddedTView(t, n) {
      return this.isApplyingToNode(t)
        ? ((this.crossesNgTemplate = !0),
          this.addMatch(-t.index, n),
          new e(this.metadata))
        : null;
    }
    isApplyingToNode(t) {
      if (this._appliesToNextNode && (this.metadata.flags & 1) !== 1) {
        let n = this._declarationNodeIndex,
          r = t.parent;
        for (; r !== null && r.type & 8 && r.index !== n; ) r = r.parent;
        return n === (r !== null ? r.index : -1);
      }
      return this._appliesToNextNode;
    }
    matchTNode(t, n) {
      let r = this.metadata.predicate;
      if (Array.isArray(r))
        for (let o = 0; o < r.length; o++) {
          let i = r[o];
          (this.matchTNodeWithReadOption(t, n, gb(n, i)),
            this.matchTNodeWithReadOption(t, n, Qi(n, t, i, !1, !1)));
        }
      else
        r === Dn
          ? n.type & 4 && this.matchTNodeWithReadOption(t, n, -1)
          : this.matchTNodeWithReadOption(t, n, Qi(n, t, r, !1, !1));
    }
    matchTNodeWithReadOption(t, n, r) {
      if (r !== null) {
        let o = this.metadata.read;
        if (o !== null) {
          if (o === tt || o === nt || (o === Dn && n.type & 4))
            this.addMatch(n.index, -2);
          else {
            let i = Qi(n, t, o, !1, !1);
            i !== null && this.addMatch(n.index, i);
          }
        } else this.addMatch(n.index, r);
      }
    }
    addMatch(t, n) {
      this.matches === null ? (this.matches = [t, n]) : this.matches.push(t, n);
    }
  };
function gb(e, t) {
  let n = e.localNames;
  if (n !== null) {
    for (let r = 0; r < n.length; r += 2) if (n[r] === t) return n[r + 1];
  }
  return null;
}
function mb(e, t) {
  return e.type & 11 ? wr(e, t) : e.type & 4 ? Js(e, t) : null;
}
function yb(e, t, n, r) {
  return n === -1 ? mb(t, e) : n === -2 ? vb(e, t, r) : uo(e, e[N], n, t);
}
function vb(e, t, n) {
  if (n === tt) return wr(t, e);
  if (n === Dn) return Js(t, e);
  if (n === nt) return Am(t, e);
}
function Om(e, t, n, r) {
  let o = t[pt].queries[r];
  if (o.matches === null) {
    let i = e.data,
      s = n.matches,
      a = [];
    for (let c = 0; s !== null && c < s.length; c += 2) {
      let u = s[c];
      if (u < 0) a.push(null);
      else {
        let l = i[u];
        a.push(yb(t, l, s[c + 1], n.metadata.read));
      }
    }
    o.matches = a;
  }
  return o.matches;
}
function Iu(e, t, n, r) {
  let o = e.queries.getByIndex(n),
    i = o.matches;
  if (i !== null) {
    let s = Om(e, t, o, n);
    for (let a = 0; a < i.length; a += 2) {
      let c = i[a];
      if (c > 0) r.push(s[a / 2]);
      else {
        let u = i[a + 1],
          l = t[-c];
        for (let d = fe; d < l.length; d++) {
          let h = l[d];
          h[fn] === h[he] && Iu(h[N], h, u, r);
        }
        if (l[or] !== null) {
          let d = l[or];
          for (let h = 0; h < d.length; h++) {
            let f = d[h];
            Iu(f[N], f, u, r);
          }
        }
      }
    }
  }
  return r;
}
function Wl(e, t) {
  return e[pt].queries[t].queryList;
}
function km(e, t, n) {
  let r = new Yc((n & 4) === 4);
  return (
    Kw(e, t, r, r.destroy),
    (t[pt] ??= new Du()).queries.push(new vu(r)) - 1
  );
}
function Pm(e, t, n) {
  let r = Q();
  return (
    r.firstCreatePass &&
      (Fm(r, new ws(e, t, n), -1), (t & 2) === 2 && (r.staticViewQueries = !0)),
    km(r, C(), t)
  );
}
function Db(e, t, n, r) {
  let o = Q();
  if (o.firstCreatePass) {
    let i = we();
    (Fm(o, new ws(t, n, r), i.index),
      Eb(o, e),
      (n & 2) === 2 && (o.staticContentQueries = !0));
  }
  return km(o, C(), n);
}
function wb(e) {
  return e.split(",").map((t) => t.trim());
}
function Fm(e, t, n) {
  (e.queries === null && (e.queries = new wu()), e.queries.track(new Eu(t, n)));
}
function Eb(e, t) {
  let n = e.contentQueries || (e.contentQueries = []),
    r = n.length ? n[n.length - 1] : -1;
  t !== r && n.push(e.queries.length - 1, t);
}
function Zl(e, t) {
  return e.queries.getByIndex(t);
}
function Lm(e, t) {
  let n = e[N],
    r = Zl(n, t);
  return r.crossesNgTemplate ? Iu(n, e, t, []) : Om(n, e, r, t);
}
function Yl(e, t, n) {
  let r,
    o = fi(() => {
      r._dirtyCounter();
      let i = Cb(r, e);
      if (t && i === void 0) throw new v(-951, !1);
      return i;
    });
  return ((r = o[re]), (r._dirtyCounter = sr(0)), (r._flatValue = void 0), o);
}
function jm(e) {
  return Yl(!0, !1, e);
}
function Vm(e) {
  return Yl(!0, !0, e);
}
function Ib(e) {
  return Yl(!1, !1, e);
}
function Bm(e, t) {
  let n = e[re];
  ((n._lView = C()),
    (n._queryIndex = t),
    (n._queryList = Wl(n._lView, t)),
    n._queryList.onDirty(() => n._dirtyCounter.update((r) => r + 1)));
}
function Cb(e, t) {
  let n = e._lView,
    r = e._queryIndex;
  if (n === void 0 || r === void 0 || n[_] & 4) return t ? void 0 : de;
  let o = Wl(n, r),
    i = Lm(n, r);
  return (
    o.reset(i, gg),
    t
      ? o.first
      : o._changesDetected || e._flatValue === void 0
        ? (e._flatValue = o.toArray())
        : e._flatValue
  );
}
function Ah(e, t) {
  return jm(t);
}
function bb(e, t) {
  return Vm(t);
}
var w1 = ((Ah.required = bb), Ah);
function Oh(e, t) {
  return jm(t);
}
function Mb(e, t) {
  return Vm(t);
}
var E1 = ((Oh.required = Mb), Oh);
function I1(e, t) {
  return Ib(t);
}
function Um(e, t) {
  let n = Object.create(ip),
    r = new ms();
  n.value = e;
  function o() {
    return (Ot(n), kh(n.value), n.value);
  }
  return (
    (o[re] = n),
    (o.asReadonly = Hs.bind(o)),
    (o.set = (i) => {
      n.equal(n.value, i) || (tn(n, i), r.emit(i));
    }),
    (o.update = (i) => {
      (kh(n.value), o.set(i(n.value)));
    }),
    (o.subscribe = r.subscribe.bind(r)),
    (o.destroyRef = r.destroyRef),
    o
  );
}
function kh(e) {
  if (e === Rs) throw new v(952, !1);
}
function Ph(e, t) {
  return Um(e, t);
}
function Sb(e) {
  return Um(Rs, e);
}
var C1 = ((Ph.required = Sb), Ph);
var En = class {},
  Ql = class {};
function $m(e, t) {
  return new Es(e, t ?? null, []);
}
var Es = class extends En {
    ngModuleType;
    _parent;
    _bootstrapComponents = [];
    _r3Injector;
    instance;
    destroyCbs = [];
    componentFactoryResolver = new Ds(this);
    constructor(t, n, r, o = !0) {
      (super(), (this.ngModuleType = t), (this._parent = n));
      let i = Ip(t);
      ((this._bootstrapComponents = $g(i.bootstrap)),
        (this._r3Injector = ug(
          t,
          n,
          [
            { provide: En, useValue: this },
            { provide: ea, useValue: this.componentFactoryResolver },
            ...r,
          ],
          Ce(t),
          new Set(["environment"]),
        )),
        o && this.resolveInjectorInitializers());
    }
    resolveInjectorInitializers() {
      (this._r3Injector.resolveInjectorInitializers(),
        (this.instance = this._r3Injector.get(this.ngModuleType)));
    }
    get injector() {
      return this._r3Injector;
    }
    destroy() {
      let t = this._r3Injector;
      (!t.destroyed && t.destroy(),
        this.destroyCbs.forEach((n) => n()),
        (this.destroyCbs = null));
    }
    onDestroy(t) {
      this.destroyCbs.push(t);
    }
  },
  Cu = class extends Ql {
    moduleType;
    constructor(t) {
      (super(), (this.moduleType = t));
    }
    create(t) {
      return new Es(this.moduleType, t, []);
    }
  };
var Is = class extends En {
  injector;
  componentFactoryResolver = new Ds(this);
  instance = null;
  constructor(t) {
    super();
    let n = new so(
      [
        ...t.providers,
        { provide: En, useValue: this },
        { provide: ea, useValue: this.componentFactoryResolver },
      ],
      t.parent || Ps(),
      t.debugName,
      new Set(["environment"]),
    );
    ((this.injector = n),
      t.runEnvironmentInitializers && n.resolveInjectorInitializers());
  }
  destroy() {
    this.injector.destroy();
  }
  onDestroy(t) {
    this.injector.onDestroy(t);
  }
};
function ta(e, t, n = null) {
  return new Is({
    providers: e,
    parent: t,
    debugName: n,
    runEnvironmentInitializers: !0,
  }).injector;
}
var Tb = (() => {
  class e {
    _injector;
    cachedInjectors = new Map();
    constructor(n) {
      this._injector = n;
    }
    getOrCreateStandaloneInjector(n) {
      if (!n.standalone) return null;
      if (!this.cachedInjectors.has(n)) {
        let r = bp(!1, n.type),
          o =
            r.length > 0
              ? ta([r], this._injector, `Standalone[${n.type.name}]`)
              : null;
        this.cachedInjectors.set(n, o);
      }
      return this.cachedInjectors.get(n);
    }
    ngOnDestroy() {
      try {
        for (let n of this.cachedInjectors.values()) n !== null && n.destroy();
      } finally {
        this.cachedInjectors.clear();
      }
    }
    static ɵprov = E({
      token: e,
      providedIn: "environment",
      factory: () => new e(b(ye)),
    });
  }
  return e;
})();
function na(e) {
  return vo(() => {
    let t = Hm(e),
      n = j(D({}, t), {
        decls: e.decls,
        vars: e.vars,
        template: e.template,
        consts: e.consts || null,
        ngContentSelectors: e.ngContentSelectors,
        onPush: e.changeDetection === yg.OnPush,
        directiveDefs: null,
        pipeDefs: null,
        dependencies: (t.standalone && e.dependencies) || null,
        getStandaloneInjector: t.standalone
          ? (o) => o.get(Tb).getOrCreateStandaloneInjector(n)
          : null,
        getExternalStyles: null,
        signals: e.signals ?? !1,
        data: e.data || {},
        encapsulation: e.encapsulation || Ke.Emulated,
        styles: e.styles || de,
        _: null,
        schemas: e.schemas || null,
        tView: null,
        id: "",
      });
    (t.standalone && zt("NgStandalone"), zm(n));
    let r = e.dependencies;
    return (
      (n.directiveDefs = Fh(r, !1)),
      (n.pipeDefs = Fh(r, !0)),
      (n.id = Ab(n)),
      n
    );
  });
}
function _b(e) {
  return $t(e) || Cp(e);
}
function xb(e) {
  return e !== null;
}
function Ir(e) {
  return vo(() => ({
    type: e.type,
    bootstrap: e.bootstrap || de,
    declarations: e.declarations || de,
    imports: e.imports || de,
    exports: e.exports || de,
    transitiveCompileScopes: null,
    schemas: e.schemas || null,
    id: e.id || null,
  }));
}
function Nb(e, t) {
  if (e == null) return We;
  let n = {};
  for (let r in e)
    if (e.hasOwnProperty(r)) {
      let o = e[r],
        i,
        s,
        a,
        c;
      (Array.isArray(o)
        ? ((a = o[0]), (i = o[1]), (s = o[2] ?? i), (c = o[3] || null))
        : ((i = o), (s = o), (a = Gs.None), (c = null)),
        (n[i] = [r, a, c]),
        (t[i] = s));
    }
  return n;
}
function Rb(e) {
  if (e == null) return We;
  let t = {};
  for (let n in e) e.hasOwnProperty(n) && (t[e[n]] = n);
  return t;
}
function rt(e) {
  return vo(() => {
    let t = Hm(e);
    return (zm(t), t);
  });
}
function ra(e) {
  return {
    type: e.type,
    name: e.name,
    factory: null,
    pure: e.pure !== !1,
    standalone: e.standalone ?? !0,
    onDestroy: e.type.prototype.ngOnDestroy || null,
  };
}
function Hm(e) {
  let t = {};
  return {
    type: e.type,
    providersResolver: null,
    factory: null,
    hostBindings: e.hostBindings || null,
    hostVars: e.hostVars || 0,
    hostAttrs: e.hostAttrs || null,
    contentQueries: e.contentQueries || null,
    declaredInputs: t,
    inputConfig: e.inputs || We,
    exportAs: e.exportAs || null,
    standalone: e.standalone ?? !0,
    signals: e.signals === !0,
    selectors: e.selectors || de,
    viewQuery: e.viewQuery || null,
    features: e.features || null,
    setInput: null,
    findHostDirectiveDefs: null,
    hostDirectives: null,
    inputs: Nb(e.inputs, t),
    outputs: Rb(e.outputs),
    debugInfo: null,
  };
}
function zm(e) {
  e.features?.forEach((t) => t(e));
}
function Fh(e, t) {
  if (!e) return null;
  let n = t ? Rw : _b;
  return () => (typeof e == "function" ? e() : e).map((r) => n(r)).filter(xb);
}
function Ab(e) {
  let t = 0,
    n = typeof e.consts == "function" ? "" : e.consts,
    r = [
      e.selectors,
      e.ngContentSelectors,
      e.hostVars,
      e.hostAttrs,
      n,
      e.vars,
      e.decls,
      e.encapsulation,
      e.standalone,
      e.signals,
      e.exportAs,
      JSON.stringify(e.inputs),
      JSON.stringify(e.outputs),
      Object.getOwnPropertyNames(e.type.prototype),
      !!e.contentQueries,
      !!e.viewQuery,
    ];
  for (let i of r.join("|")) t = (Math.imul(31, t) + i.charCodeAt(0)) << 0;
  return ((t += 2147483648), "c" + t);
}
function Ob(e) {
  return Object.getPrototypeOf(e.prototype).constructor;
}
function kb(e) {
  let t = Ob(e.type),
    n = !0,
    r = [e];
  for (; t; ) {
    let o;
    if (Ze(e)) o = t.ɵcmp || t.ɵdir;
    else {
      if (t.ɵcmp) throw new v(903, !1);
      o = t.ɵdir;
    }
    if (o) {
      if (n) {
        r.push(o);
        let s = e;
        ((s.inputs = Ic(e.inputs)),
          (s.declaredInputs = Ic(e.declaredInputs)),
          (s.outputs = Ic(e.outputs)));
        let a = o.hostBindings;
        a && Vb(e, a);
        let c = o.viewQuery,
          u = o.contentQueries;
        if (
          (c && Lb(e, c),
          u && jb(e, u),
          Pb(e, o),
          aw(e.outputs, o.outputs),
          Ze(o) && o.data.animation)
        ) {
          let l = e.data;
          l.animation = (l.animation || []).concat(o.data.animation);
        }
      }
      let i = o.features;
      if (i)
        for (let s = 0; s < i.length; s++) {
          let a = i[s];
          (a && a.ngInherit && a(e), a === kb && (n = !1));
        }
    }
    t = Object.getPrototypeOf(t);
  }
  Fb(r);
}
function Pb(e, t) {
  for (let n in t.inputs) {
    if (!t.inputs.hasOwnProperty(n) || e.inputs.hasOwnProperty(n)) continue;
    let r = t.inputs[n];
    r !== void 0 &&
      ((e.inputs[n] = r), (e.declaredInputs[n] = t.declaredInputs[n]));
  }
}
function Fb(e) {
  let t = 0,
    n = null;
  for (let r = e.length - 1; r >= 0; r--) {
    let o = e[r];
    ((o.hostVars = t += o.hostVars),
      (o.hostAttrs = ir(o.hostAttrs, (n = ir(n, o.hostAttrs)))));
  }
}
function Ic(e) {
  return e === We ? {} : e === de ? [] : e;
}
function Lb(e, t) {
  let n = e.viewQuery;
  n
    ? (e.viewQuery = (r, o) => {
        (t(r, o), n(r, o));
      })
    : (e.viewQuery = t);
}
function jb(e, t) {
  let n = e.contentQueries;
  n
    ? (e.contentQueries = (r, o, i) => {
        (t(r, o, i), n(r, o, i));
      })
    : (e.contentQueries = t);
}
function Vb(e, t) {
  let n = e.hostBindings;
  n
    ? (e.hostBindings = (r, o) => {
        (t(r, o), n(r, o));
      })
    : (e.hostBindings = t);
}
function S1(e) {
  let t = (n) => {
    let r = Array.isArray(e);
    n.hostDirectives === null
      ? ((n.findHostDirectiveDefs = qm),
        (n.hostDirectives = r ? e.map(bu) : [e]))
      : r
        ? n.hostDirectives.unshift(...e.map(bu))
        : n.hostDirectives.unshift(e);
  };
  return ((t.ngInherit = !0), t);
}
function qm(e, t, n) {
  if (e.hostDirectives !== null)
    for (let r of e.hostDirectives)
      if (typeof r == "function") {
        let o = r();
        for (let i of o) Lh(bu(i), t, n);
      } else Lh(r, t, n);
}
function Lh(e, t, n) {
  let r = Cp(e.directive);
  (Bb(r.declaredInputs, e.inputs), qm(r, t, n), n.set(r, e), t.push(r));
}
function bu(e) {
  return typeof e == "function"
    ? { directive: ae(e), inputs: We, outputs: We }
    : {
        directive: ae(e.directive),
        inputs: jh(e.inputs),
        outputs: jh(e.outputs),
      };
}
function jh(e) {
  if (e === void 0 || e.length === 0) return We;
  let t = {};
  for (let n = 0; n < e.length; n += 2) t[e[n]] = e[n + 1];
  return t;
}
function Bb(e, t) {
  for (let n in t)
    if (t.hasOwnProperty(n)) {
      let r = t[n],
        o = e[n];
      e[r] = o;
    }
}
function Gm(e) {
  return Kl(e)
    ? Array.isArray(e) || (!(e instanceof Map) && Symbol.iterator in e)
    : !1;
}
function Ub(e, t) {
  if (Array.isArray(e)) for (let n = 0; n < e.length; n++) t(e[n]);
  else {
    let n = e[Symbol.iterator](),
      r;
    for (; !(r = n.next()).done; ) t(r.value);
  }
}
function Kl(e) {
  return e !== null && (typeof e == "function" || typeof e == "object");
}
function Sn(e, t, n) {
  return (e[t] = n);
}
function Wm(e, t) {
  return e[t];
}
function ve(e, t, n) {
  let r = e[t];
  return Object.is(r, n) ? !1 : ((e[t] = n), !0);
}
function po(e, t, n, r) {
  let o = ve(e, t, n);
  return ve(e, t + 1, r) || o;
}
function $b(e, t, n, r, o) {
  let i = po(e, t, n, r);
  return ve(e, t + 2, o) || i;
}
function Zm(e, t, n, r, o, i) {
  let s = po(e, t, n, r);
  return po(e, t + 2, o, i) || s;
}
function Hb(e, t, n, r, o, i, s, a, c) {
  let u = t.consts,
    l = No(t, e, 4, s || null, a || null);
  (il() && Gl(t, n, l, gt(u, c), Fl),
    (l.mergedAttrs = ir(l.mergedAttrs, l.attrs)),
    fl(t, l));
  let d = (l.tView = Rl(
    2,
    l,
    r,
    o,
    i,
    t.directiveRegistry,
    t.pipeRegistry,
    null,
    t.schemas,
    u,
    null,
  ));
  return (
    t.queries !== null &&
      (t.queries.template(t, l), (d.queries = t.queries.embeddedTView(l))),
    l
  );
}
function Cs(e, t, n, r, o, i, s, a, c, u) {
  let l = n + Y,
    d = t.firstCreatePass ? Hb(l, t, e, r, o, i, s, a, c) : t.data[l];
  Qe(d, !1);
  let h = qb(t, e, d, n);
  (Co() && Qs(t, e, h, d), Ht(h, e));
  let f = ym(h, e, h, d);
  return (
    (e[l] = f),
    Ol(e, f),
    hb(f, d, e),
    Ls(d) && Ws(t, e, d),
    c != null && Pl(e, d, u),
    d
  );
}
function zb(e, t, n, r, o, i, s, a) {
  let c = C(),
    u = Q(),
    l = gt(u.consts, i);
  return (Cs(c, u, e, t, n, r, o, l, s, a), zb);
}
var qb = Gb;
function Gb(e, t, n, r) {
  return (bo(!0), t[U].createComment(""));
}
var Jl = (() => {
  class e {
    log(n) {
      console.log(n);
    }
    warn(n) {
      console.warn(n);
    }
    static ɵfac = function (r) {
      return new (r || e)();
    };
    static ɵprov = E({ token: e, factory: e.ɵfac, providedIn: "platform" });
  }
  return e;
})();
var Ym = new I("");
var Qm = (() => {
    class e {
      static ɵprov = E({
        token: e,
        providedIn: "root",
        factory: () => new Mu(),
      });
    }
    return e;
  })(),
  Mu = class {
    queuedEffectCount = 0;
    queues = new Map();
    schedule(t) {
      this.enqueue(t);
    }
    remove(t) {
      let n = t.zone,
        r = this.queues.get(n);
      r.has(t) && (r.delete(t), this.queuedEffectCount--);
    }
    enqueue(t) {
      let n = t.zone;
      this.queues.has(n) || this.queues.set(n, new Set());
      let r = this.queues.get(n);
      r.has(t) || (this.queuedEffectCount++, r.add(t));
    }
    flush() {
      for (; this.queuedEffectCount > 0; )
        for (let [t, n] of this.queues)
          t === null ? this.flushQueue(n) : t.run(() => this.flushQueue(n));
    }
    flushQueue(t) {
      for (let n of t) (t.delete(n), this.queuedEffectCount--, n.run());
    }
  };
function Cr(e) {
  return !!e && typeof e.then == "function";
}
function Xl(e) {
  return !!e && typeof e.subscribe == "function";
}
var oa = new I("");
var Km = (() => {
    class e {
      resolve;
      reject;
      initialized = !1;
      done = !1;
      donePromise = new Promise((n, r) => {
        ((this.resolve = n), (this.reject = r));
      });
      appInits = m(oa, { optional: !0 }) ?? [];
      injector = m(ce);
      constructor() {}
      runInitializers() {
        if (this.initialized) return;
        let n = [];
        for (let o of this.appInits) {
          let i = Me(this.injector, o);
          if (Cr(i)) n.push(i);
          else if (Xl(i)) {
            let s = new Promise((a, c) => {
              i.subscribe({ complete: a, error: c });
            });
            n.push(s);
          }
        }
        let r = () => {
          ((this.done = !0), this.resolve());
        };
        (Promise.all(n)
          .then(() => {
            r();
          })
          .catch((o) => {
            this.reject(o);
          }),
          n.length === 0 && r(),
          (this.initialized = !0));
      }
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵprov = E({ token: e, factory: e.ɵfac, providedIn: "root" });
    }
    return e;
  })(),
  Ro = new I("");
function Wb() {
  Ga(() => {
    throw new v(600, !1);
  });
}
function Zb(e) {
  return e.isBoundToModule;
}
var Yb = 10;
var vt = (() => {
  class e {
    _runningTick = !1;
    _destroyed = !1;
    _destroyListeners = [];
    _views = [];
    internalErrorHandler = m(LE);
    afterRenderManager = m(Cl);
    zonelessEnabled = m($s);
    rootEffectScheduler = m(Qm);
    dirtyFlags = 0;
    tracingSnapshot = null;
    externalTestViews = new Set();
    afterTick = new X();
    get allViews() {
      return [...this.externalTestViews.keys(), ...this._views];
    }
    get destroyed() {
      return this._destroyed;
    }
    componentTypes = [];
    components = [];
    isStable = m(Ct).hasPendingTasks.pipe(O((n) => !n));
    constructor() {
      m(Mn, { optional: !0 });
    }
    whenStable() {
      let n;
      return new Promise((r) => {
        n = this.isStable.subscribe({
          next: (o) => {
            o && r();
          },
        });
      }).finally(() => {
        n.unsubscribe();
      });
    }
    _injector = m(ye);
    _rendererFactory = null;
    get injector() {
      return this._injector;
    }
    bootstrap(n, r) {
      $(10);
      let o = n instanceof Sm;
      if (!this._injector.get(Km).done) {
        let h = "";
        throw new v(405, h);
      }
      let s;
      (o ? (s = n) : (s = this._injector.get(ea).resolveComponentFactory(n)),
        this.componentTypes.push(s.componentType));
      let a = Zb(s) ? void 0 : this._injector.get(En),
        c = r || s.selector,
        u = s.create(ce.NULL, [], c, a),
        l = u.location.nativeElement,
        d = u.injector.get(Ym, null);
      return (
        d?.registerApplication(l),
        u.onDestroy(() => {
          (this.detachView(u.hostView),
            Ki(this.components, u),
            d?.unregisterApplication(l));
        }),
        this._loadComponent(u),
        $(11, u),
        u
      );
    }
    tick() {
      (this.zonelessEnabled || (this.dirtyFlags |= 1), this._tick());
    }
    _tick() {
      ($(12),
        this.tracingSnapshot !== null
          ? this.tracingSnapshot.run(Il.CHANGE_DETECTION, this.tickImpl)
          : this.tickImpl());
    }
    tickImpl = () => {
      if (this._runningTick) throw new v(101, !1);
      let n = A(null);
      try {
        ((this._runningTick = !0), this.synchronize());
      } catch (r) {
        this.internalErrorHandler(r);
      } finally {
        ((this._runningTick = !1),
          this.tracingSnapshot?.dispose(),
          (this.tracingSnapshot = null),
          A(n),
          this.afterTick.next(),
          $(13));
      }
    };
    synchronize() {
      this._rendererFactory === null &&
        !this._injector.destroyed &&
        (this._rendererFactory = this._injector.get(ur, null, {
          optional: !0,
        }));
      let n = 0;
      for (; this.dirtyFlags !== 0 && n++ < Yb; )
        ($(14), this.synchronizeOnce(), $(15));
    }
    synchronizeOnce() {
      if (
        (this.dirtyFlags & 16 &&
          ((this.dirtyFlags &= -17), this.rootEffectScheduler.flush()),
        this.dirtyFlags & 7)
      ) {
        let n = !!(this.dirtyFlags & 1);
        ((this.dirtyFlags &= -8), (this.dirtyFlags |= 8));
        for (let { _lView: r, notifyErrorHandler: o } of this.allViews)
          Qb(r, o, n, this.zonelessEnabled);
        if (
          ((this.dirtyFlags &= -5),
          this.syncDirtyFlagsWithViews(),
          this.dirtyFlags & 23)
        )
          return;
      } else (this._rendererFactory?.begin?.(), this._rendererFactory?.end?.());
      (this.dirtyFlags & 8 &&
        ((this.dirtyFlags &= -9), this.afterRenderManager.execute()),
        this.syncDirtyFlagsWithViews());
    }
    syncDirtyFlagsWithViews() {
      if (this.allViews.some(({ _lView: n }) => Vs(n))) {
        this.dirtyFlags |= 2;
        return;
      } else this.dirtyFlags &= -8;
    }
    attachView(n) {
      let r = n;
      (this._views.push(r), r.attachToAppRef(this));
    }
    detachView(n) {
      let r = n;
      (Ki(this._views, r), r.detachFromAppRef());
    }
    _loadComponent(n) {
      (this.attachView(n.hostView),
        this.tick(),
        this.components.push(n),
        this._injector.get(Ro, []).forEach((o) => o(n)));
    }
    ngOnDestroy() {
      if (!this._destroyed)
        try {
          (this._destroyListeners.forEach((n) => n()),
            this._views.slice().forEach((n) => n.destroy()));
        } finally {
          ((this._destroyed = !0),
            (this._views = []),
            (this._destroyListeners = []));
        }
    }
    onDestroy(n) {
      return (
        this._destroyListeners.push(n),
        () => Ki(this._destroyListeners, n)
      );
    }
    destroy() {
      if (this._destroyed) throw new v(406, !1);
      let n = this._injector;
      n.destroy && !n.destroyed && n.destroy();
    }
    get viewCount() {
      return this._views.length;
    }
    static ɵfac = function (r) {
      return new (r || e)();
    };
    static ɵprov = E({ token: e, factory: e.ɵfac, providedIn: "root" });
  }
  return e;
})();
function Ki(e, t) {
  let n = e.indexOf(t);
  n > -1 && e.splice(n, 1);
}
function Qb(e, t, n, r) {
  if (!n && !Vs(e)) return;
  hm(e, t, n && !r ? 0 : 1);
}
function ed(e, t, n, r) {
  let o = C(),
    i = Cn();
  if (ve(o, i, t)) {
    let s = Q(),
      a = Us();
    GI(a, o, e, t, n, r);
  }
  return ed;
}
function td(e, t, n, r) {
  return ve(e, Cn(), n) ? t + Ut(n) + r : Se;
}
function Kb(e, t, n, r, o, i) {
  let s = zp(),
    a = po(e, s, n, o);
  return (cl(2), a ? t + Ut(n) + r + Ut(o) + i : Se);
}
function Gi(e, t) {
  return (e << 17) | (t << 2);
}
function In(e) {
  return (e >> 17) & 32767;
}
function Jb(e) {
  return (e & 2) == 2;
}
function Xb(e, t) {
  return (e & 131071) | (t << 17);
}
function Su(e) {
  return e | 2;
}
function lr(e) {
  return (e & 131068) >> 2;
}
function Cc(e, t) {
  return (e & -131069) | (t << 2);
}
function eM(e) {
  return (e & 1) === 1;
}
function Tu(e) {
  return e | 1;
}
function tM(e, t, n, r, o, i) {
  let s = i ? t.classBindings : t.styleBindings,
    a = In(s),
    c = lr(s);
  e[r] = n;
  let u = !1,
    l;
  if (Array.isArray(n)) {
    let d = n;
    ((l = d[1]), (l === null || wo(d, l) > 0) && (u = !0));
  } else l = n;
  if (o) {
    if (c !== 0) {
      let h = In(e[a + 1]);
      ((e[r + 1] = Gi(h, a)),
        h !== 0 && (e[h + 1] = Cc(e[h + 1], r)),
        (e[a + 1] = Xb(e[a + 1], r)));
    } else
      ((e[r + 1] = Gi(a, 0)), a !== 0 && (e[a + 1] = Cc(e[a + 1], r)), (a = r));
  } else
    ((e[r + 1] = Gi(c, 0)),
      a === 0 ? (a = r) : (e[c + 1] = Cc(e[c + 1], r)),
      (c = r));
  (u && (e[r + 1] = Su(e[r + 1])),
    Vh(e, l, r, !0),
    Vh(e, l, r, !1),
    nM(t, l, e, r, i),
    (s = Gi(a, c)),
    i ? (t.classBindings = s) : (t.styleBindings = s));
}
function nM(e, t, n, r, o) {
  let i = o ? e.residualClasses : e.residualStyles;
  i != null &&
    typeof t == "string" &&
    wo(i, t) >= 0 &&
    (n[r + 1] = Tu(n[r + 1]));
}
function Vh(e, t, n, r) {
  let o = e[n + 1],
    i = t === null,
    s = r ? In(o) : lr(o),
    a = !1;
  for (; s !== 0 && (a === !1 || i); ) {
    let c = e[s],
      u = e[s + 1];
    (rM(c, t) && ((a = !0), (e[s + 1] = r ? Tu(u) : Su(u))),
      (s = r ? In(u) : lr(u)));
  }
  a && (e[n + 1] = r ? Su(o) : Tu(o));
}
function rM(e, t) {
  return e === null || t == null || (Array.isArray(e) ? e[1] : e) === t
    ? !0
    : Array.isArray(e) && typeof t == "string"
      ? wo(e, t) >= 0
      : !1;
}
var te = { textEnd: 0, key: 0, keyEnd: 0, value: 0, valueEnd: 0 };
function Jm(e) {
  return e.substring(te.key, te.keyEnd);
}
function oM(e) {
  return e.substring(te.value, te.valueEnd);
}
function iM(e) {
  return (ty(e), Xm(e, dr(e, 0, te.textEnd)));
}
function Xm(e, t) {
  let n = te.textEnd;
  return n === t ? -1 : ((t = te.keyEnd = aM(e, (te.key = t), n)), dr(e, t, n));
}
function sM(e) {
  return (ty(e), ey(e, dr(e, 0, te.textEnd)));
}
function ey(e, t) {
  let n = te.textEnd,
    r = (te.key = dr(e, t, n));
  return n === r
    ? -1
    : ((r = te.keyEnd = cM(e, r, n)),
      (r = Bh(e, r, n, 58)),
      (r = te.value = dr(e, r, n)),
      (r = te.valueEnd = uM(e, r, n)),
      Bh(e, r, n, 59));
}
function ty(e) {
  ((te.key = 0),
    (te.keyEnd = 0),
    (te.value = 0),
    (te.valueEnd = 0),
    (te.textEnd = e.length));
}
function dr(e, t, n) {
  for (; t < n && e.charCodeAt(t) <= 32; ) t++;
  return t;
}
function aM(e, t, n) {
  for (; t < n && e.charCodeAt(t) > 32; ) t++;
  return t;
}
function cM(e, t, n) {
  let r;
  for (
    ;
    t < n &&
    ((r = e.charCodeAt(t)) === 45 ||
      r === 95 ||
      ((r & -33) >= 65 && (r & -33) <= 90) ||
      (r >= 48 && r <= 57));
  )
    t++;
  return t;
}
function Bh(e, t, n, r) {
  return ((t = dr(e, t, n)), t < n && t++, t);
}
function uM(e, t, n) {
  let r = -1,
    o = -1,
    i = -1,
    s = t,
    a = s;
  for (; s < n; ) {
    let c = e.charCodeAt(s++);
    if (c === 59) return a;
    (c === 34 || c === 39
      ? (a = s = Uh(e, c, s, n))
      : t === s - 4 && i === 85 && o === 82 && r === 76 && c === 40
        ? (a = s = Uh(e, 41, s, n))
        : c > 32 && (a = s),
      (i = o),
      (o = r),
      (r = c & -33));
  }
  return a;
}
function Uh(e, t, n, r) {
  let o = -1,
    i = n;
  for (; i < r; ) {
    let s = e.charCodeAt(i++);
    if (s == t && o !== 92) return i;
    s == 92 && o === 92 ? (o = 0) : (o = s);
  }
  throw new Error();
}
function ia(e, t, n) {
  let r = C(),
    o = Cn();
  if (ve(r, o, t)) {
    let i = Q(),
      s = Us();
    Zs(i, s, r, e, t, r[U], n, !1);
  }
  return ia;
}
function _u(e, t, n, r, o) {
  jl(t, e, n, o ? "class" : "style", r);
}
function ny(e, t, n) {
  return (oy(e, t, n, !1), ny);
}
function nd(e, t) {
  return (oy(e, t, null, !0), nd);
}
function T1(e) {
  rd(ay, lM, e, !1);
}
function lM(e, t) {
  for (let n = sM(t); n >= 0; n = ey(t, n)) ay(e, Jm(t), oM(t));
}
function _1(e) {
  rd(mM, ry, e, !0);
}
function ry(e, t) {
  for (let n = iM(t); n >= 0; n = Xm(t, n)) Do(e, Jm(t), !0);
}
function oy(e, t, n, r) {
  let o = C(),
    i = Q(),
    s = cl(2);
  if ((i.firstUpdatePass && sy(i, e, s, r), t !== Se && ve(o, s, t))) {
    let a = i.data[Et()];
    cy(i, a, o, o[U], e, (o[s + 1] = vM(t, n)), r, s);
  }
}
function rd(e, t, n, r) {
  let o = Q(),
    i = cl(2);
  o.firstUpdatePass && sy(o, null, i, r);
  let s = C();
  if (n !== Se && ve(s, i, n)) {
    let a = o.data[Et()];
    if (uy(a, r) && !iy(o, i)) {
      let c = r ? a.classesWithoutHost : a.stylesWithoutHost;
      (c !== null && (n = Rc(c, n || "")), _u(o, a, s, n, r));
    } else yM(o, a, s, s[U], s[i + 1], (s[i + 1] = gM(e, t, n)), r, i);
  }
}
function iy(e, t) {
  return t >= e.expandoStartIndex;
}
function sy(e, t, n, r) {
  let o = e.data;
  if (o[n + 1] === null) {
    let i = o[Et()],
      s = iy(e, n);
    (uy(i, r) && t === null && !s && (t = !1),
      (t = dM(o, i, t, r)),
      tM(o, i, t, n, s, r));
  }
}
function dM(e, t, n, r) {
  let o = cE(e),
    i = r ? t.residualClasses : t.residualStyles;
  if (o === null)
    (r ? t.classBindings : t.styleBindings) === 0 &&
      ((n = bc(null, e, t, n, r)), (n = go(n, t.attrs, r)), (i = null));
  else {
    let s = t.directiveStylingLast;
    if (s === -1 || e[s] !== o)
      if (((n = bc(o, e, t, n, r)), i === null)) {
        let c = fM(e, t, r);
        c !== void 0 &&
          Array.isArray(c) &&
          ((c = bc(null, e, t, c[1], r)),
          (c = go(c, t.attrs, r)),
          hM(e, t, r, c));
      } else i = pM(e, t, r);
  }
  return (
    i !== void 0 && (r ? (t.residualClasses = i) : (t.residualStyles = i)),
    n
  );
}
function fM(e, t, n) {
  let r = n ? t.classBindings : t.styleBindings;
  if (lr(r) !== 0) return e[In(r)];
}
function hM(e, t, n, r) {
  let o = n ? t.classBindings : t.styleBindings;
  e[In(o)] = r;
}
function pM(e, t, n) {
  let r,
    o = t.directiveEnd;
  for (let i = 1 + t.directiveStylingLast; i < o; i++) {
    let s = e[i].hostAttrs;
    r = go(r, s, n);
  }
  return go(r, t.attrs, n);
}
function bc(e, t, n, r, o) {
  let i = null,
    s = n.directiveEnd,
    a = n.directiveStylingLast;
  for (
    a === -1 ? (a = n.directiveStart) : a++;
    a < s && ((i = t[a]), (r = go(r, i.hostAttrs, o)), i !== e);
  )
    a++;
  return (e !== null && (n.directiveStylingLast = a), r);
}
function go(e, t, n) {
  let r = n ? 1 : 2,
    o = -1;
  if (t !== null)
    for (let i = 0; i < t.length; i++) {
      let s = t[i];
      typeof s == "number"
        ? (o = s)
        : o === r &&
          (Array.isArray(e) || (e = e === void 0 ? [] : ["", e]),
          Do(e, s, n ? !0 : t[++i]));
    }
  return e === void 0 ? null : e;
}
function gM(e, t, n) {
  if (n == null || n === "") return de;
  let r = [],
    o = xe(n);
  if (Array.isArray(o)) for (let i = 0; i < o.length; i++) e(r, o[i], !0);
  else if (typeof o == "object")
    for (let i in o) o.hasOwnProperty(i) && e(r, i, o[i]);
  else typeof o == "string" && t(r, o);
  return r;
}
function ay(e, t, n) {
  Do(e, t, xe(n));
}
function mM(e, t, n) {
  let r = String(t);
  r !== "" && !r.includes(" ") && Do(e, r, n);
}
function yM(e, t, n, r, o, i, s, a) {
  o === Se && (o = de);
  let c = 0,
    u = 0,
    l = 0 < o.length ? o[0] : null,
    d = 0 < i.length ? i[0] : null;
  for (; l !== null || d !== null; ) {
    let h = c < o.length ? o[c + 1] : void 0,
      f = u < i.length ? i[u + 1] : void 0,
      p = null,
      g;
    (l === d
      ? ((c += 2), (u += 2), h !== f && ((p = d), (g = f)))
      : d === null || (l !== null && l < d)
        ? ((c += 2), (p = l))
        : ((u += 2), (p = d), (g = f)),
      p !== null && cy(e, t, n, r, p, g, s, a),
      (l = c < o.length ? o[c] : null),
      (d = u < i.length ? i[u] : null));
  }
}
function cy(e, t, n, r, o, i, s, a) {
  if (!(t.type & 3)) return;
  let c = e.data,
    u = c[a + 1],
    l = eM(u) ? $h(c, t, n, o, lr(u), s) : void 0;
  if (!bs(l)) {
    bs(i) || (Jb(u) && (i = $h(c, null, n, o, a, s)));
    let d = js(Et(), n);
    sC(r, s, d, o, i);
  }
}
function $h(e, t, n, r, o, i) {
  let s = t === null,
    a;
  for (; o > 0; ) {
    let c = e[o],
      u = Array.isArray(c),
      l = u ? c[1] : c,
      d = l === null,
      h = n[o + 1];
    h === Se && (h = d ? de : void 0);
    let f = d ? pc(h, r) : l === r ? h : void 0;
    if ((u && !bs(f) && (f = pc(c, r)), bs(f) && ((a = f), s))) return a;
    let p = e[o + 1];
    o = s ? In(p) : lr(p);
  }
  if (t !== null) {
    let c = i ? t.residualClasses : t.residualStyles;
    c != null && (a = pc(c, r));
  }
  return a;
}
function bs(e) {
  return e !== void 0;
}
function vM(e, t) {
  return (
    e == null ||
      e === "" ||
      (typeof t == "string"
        ? (e = e + t)
        : typeof e == "object" && (e = Ce(xe(e)))),
    e
  );
}
function uy(e, t) {
  return (e.flags & (t ? 8 : 16)) !== 0;
}
function x1(e, t, n) {
  let r = C(),
    o = td(r, e, t, n);
  rd(Do, ry, o, !0);
}
var xu = class {
  destroy(t) {}
  updateValue(t, n) {}
  swap(t, n) {
    let r = Math.min(t, n),
      o = Math.max(t, n),
      i = this.detach(o);
    if (o - r > 1) {
      let s = this.detach(r);
      (this.attach(r, i), this.attach(o, s));
    } else this.attach(r, i);
  }
  move(t, n) {
    this.attach(n, this.detach(t));
  }
};
function Mc(e, t, n, r, o) {
  return e === n && Object.is(t, r) ? 1 : Object.is(o(e, t), o(n, r)) ? -1 : 0;
}
function DM(e, t, n) {
  let r,
    o,
    i = 0,
    s = e.length - 1,
    a = void 0;
  if (Array.isArray(t)) {
    let c = t.length - 1;
    for (; i <= s && i <= c; ) {
      let u = e.at(i),
        l = t[i],
        d = Mc(i, u, i, l, n);
      if (d !== 0) {
        (d < 0 && e.updateValue(i, l), i++);
        continue;
      }
      let h = e.at(s),
        f = t[c],
        p = Mc(s, h, c, f, n);
      if (p !== 0) {
        (p < 0 && e.updateValue(s, f), s--, c--);
        continue;
      }
      let g = n(i, u),
        y = n(s, h),
        w = n(i, l);
      if (Object.is(w, y)) {
        let L = n(c, f);
        (Object.is(L, g)
          ? (e.swap(i, s), e.updateValue(s, f), c--, s--)
          : e.move(s, i),
          e.updateValue(i, l),
          i++);
        continue;
      }
      if (((r ??= new Ms()), (o ??= zh(e, i, s, n)), Nu(e, r, i, w)))
        (e.updateValue(i, l), i++, s++);
      else if (o.has(w)) (r.set(g, e.detach(i)), s--);
      else {
        let L = e.create(i, t[i]);
        (e.attach(i, L), i++, s++);
      }
    }
    for (; i <= c; ) (Hh(e, r, n, i, t[i]), i++);
  } else if (t != null) {
    let c = t[Symbol.iterator](),
      u = c.next();
    for (; !u.done && i <= s; ) {
      let l = e.at(i),
        d = u.value,
        h = Mc(i, l, i, d, n);
      if (h !== 0) (h < 0 && e.updateValue(i, d), i++, (u = c.next()));
      else {
        ((r ??= new Ms()), (o ??= zh(e, i, s, n)));
        let f = n(i, d);
        if (Nu(e, r, i, f)) (e.updateValue(i, d), i++, s++, (u = c.next()));
        else if (!o.has(f))
          (e.attach(i, e.create(i, d)), i++, s++, (u = c.next()));
        else {
          let p = n(i, l);
          (r.set(p, e.detach(i)), s--);
        }
      }
    }
    for (; !u.done; ) (Hh(e, r, n, e.length, u.value), (u = c.next()));
  }
  for (; i <= s; ) e.destroy(e.detach(s--));
  r?.forEach((c) => {
    e.destroy(c);
  });
}
function Nu(e, t, n, r) {
  return t !== void 0 && t.has(r)
    ? (e.attach(n, t.get(r)), t.delete(r), !0)
    : !1;
}
function Hh(e, t, n, r, o) {
  if (Nu(e, t, r, n(r, o))) e.updateValue(r, o);
  else {
    let i = e.create(r, o);
    e.attach(r, i);
  }
}
function zh(e, t, n, r) {
  let o = new Set();
  for (let i = t; i <= n; i++) o.add(r(i, e.at(i)));
  return o;
}
var Ms = class {
  kvMap = new Map();
  _vMap = void 0;
  has(t) {
    return this.kvMap.has(t);
  }
  delete(t) {
    if (!this.has(t)) return !1;
    let n = this.kvMap.get(t);
    return (
      this._vMap !== void 0 && this._vMap.has(n)
        ? (this.kvMap.set(t, this._vMap.get(n)), this._vMap.delete(n))
        : this.kvMap.delete(t),
      !0
    );
  }
  get(t) {
    return this.kvMap.get(t);
  }
  set(t, n) {
    if (this.kvMap.has(t)) {
      let r = this.kvMap.get(t);
      this._vMap === void 0 && (this._vMap = new Map());
      let o = this._vMap;
      for (; o.has(r); ) r = o.get(r);
      o.set(r, n);
    } else this.kvMap.set(t, n);
  }
  forEach(t) {
    for (let [n, r] of this.kvMap)
      if ((t(r, n), this._vMap !== void 0)) {
        let o = this._vMap;
        for (; o.has(r); ) ((r = o.get(r)), t(r, n));
      }
  }
};
function N1(e, t) {
  zt("NgControlFlow");
  let n = C(),
    r = Cn(),
    o = n[r] !== Se ? n[r] : -1,
    i = o !== -1 ? Ss(n, Y + o) : void 0,
    s = 0;
  if (ve(n, r, e)) {
    let a = A(null);
    try {
      if ((i !== void 0 && Dm(i, s), e !== -1)) {
        let c = Y + e,
          u = Ss(n, c),
          l = ku(n[N], c),
          d = cr(u, l.tView.ssrId),
          h = _o(n, l, t, { dehydratedView: d });
        xo(u, h, s, ar(l, d));
      }
    } finally {
      A(a);
    }
  } else if (i !== void 0) {
    let a = vm(i, s);
    a !== void 0 && (a[ne] = t);
  }
}
var Ru = class {
  lContainer;
  $implicit;
  $index;
  constructor(t, n, r) {
    ((this.lContainer = t), (this.$implicit = n), (this.$index = r));
  }
  get $count() {
    return this.lContainer.length - fe;
  }
};
function R1(e) {
  return e;
}
function A1(e, t) {
  return t;
}
var Au = class {
  hasEmptyBlock;
  trackByFn;
  liveCollection;
  constructor(t, n, r) {
    ((this.hasEmptyBlock = t), (this.trackByFn = n), (this.liveCollection = r));
  }
};
function O1(e, t, n, r, o, i, s, a, c, u, l, d, h) {
  zt("NgControlFlow");
  let f = C(),
    p = Q(),
    g = c !== void 0,
    y = C(),
    w = a ? s.bind(y[be][ne]) : s,
    L = new Au(g, w);
  ((y[Y + e] = L),
    Cs(f, p, e + 1, t, n, r, o, gt(p.consts, i)),
    g && Cs(f, p, e + 2, c, u, l, d, gt(p.consts, h)));
}
var Ou = class extends xu {
  lContainer;
  hostLView;
  templateTNode;
  operationsCounter = void 0;
  needsIndexUpdate = !1;
  constructor(t, n, r) {
    (super(),
      (this.lContainer = t),
      (this.hostLView = n),
      (this.templateTNode = r));
  }
  get length() {
    return this.lContainer.length - fe;
  }
  at(t) {
    return this.getLView(t)[ne].$implicit;
  }
  attach(t, n) {
    let r = n[nr];
    ((this.needsIndexUpdate ||= t !== this.length),
      xo(this.lContainer, n, t, ar(this.templateTNode, r)));
  }
  detach(t) {
    return (
      (this.needsIndexUpdate ||= t !== this.length - 1),
      wM(this.lContainer, t)
    );
  }
  create(t, n) {
    let r = cr(this.lContainer, this.templateTNode.tView.ssrId),
      o = _o(
        this.hostLView,
        this.templateTNode,
        new Ru(this.lContainer, n, t),
        { dehydratedView: r },
      );
    return (this.operationsCounter?.recordCreate(), o);
  }
  destroy(t) {
    (Ys(t[N], t), this.operationsCounter?.recordDestroy());
  }
  updateValue(t, n) {
    this.getLView(t)[ne].$implicit = n;
  }
  reset() {
    ((this.needsIndexUpdate = !1), this.operationsCounter?.reset());
  }
  updateIndexes() {
    if (this.needsIndexUpdate)
      for (let t = 0; t < this.length; t++) this.getLView(t)[ne].$index = t;
  }
  getLView(t) {
    return EM(this.lContainer, t);
  }
};
function k1(e) {
  let t = A(null),
    n = Et();
  try {
    let r = C(),
      o = r[N],
      i = r[n],
      s = n + 1,
      a = Ss(r, s);
    if (i.liveCollection === void 0) {
      let u = ku(o, s);
      i.liveCollection = new Ou(a, r, u);
    } else i.liveCollection.reset();
    let c = i.liveCollection;
    if ((DM(c, e, i.trackByFn), c.updateIndexes(), i.hasEmptyBlock)) {
      let u = Cn(),
        l = c.length === 0;
      if (ve(r, u, l)) {
        let d = n + 2,
          h = Ss(r, d);
        if (l) {
          let f = ku(o, d),
            p = cr(h, f.tView.ssrId),
            g = _o(r, f, void 0, { dehydratedView: p });
          xo(h, g, 0, ar(f, p));
        } else Dm(h, 0);
      }
    }
  } finally {
    A(t);
  }
}
function Ss(e, t) {
  return e[t];
}
function wM(e, t) {
  return fo(e, t);
}
function EM(e, t) {
  return vm(e, t);
}
function ku(e, t) {
  return rl(e, t);
}
function Ao(e, t, n, r) {
  let o = C(),
    i = Q(),
    s = Y + e,
    a = o[U],
    c = i.firstCreatePass ? xm(s, i, o, t, Fl, il(), n, r) : i.data[s],
    u = IM(i, o, c, a, t, e);
  o[s] = u;
  let l = Ls(c);
  return (
    Qe(c, !0),
    Qg(a, u, c),
    !Ul(c) && Co() && Qs(i, o, u, c),
    (Jw() === 0 || l) && Ht(u, o),
    Xw(),
    l && (Ws(i, o, c), Ml(i, c, o)),
    r !== null && Pl(o, c),
    Ao
  );
}
function sa() {
  let e = we();
  sl() ? al() : ((e = e.parent), Qe(e, !1));
  let t = e;
  (tE(t) && nE(), eE());
  let n = Q();
  return (
    n.firstCreatePass && Nm(n, t),
    t.classesWithoutHost != null &&
      gE(t) &&
      _u(n, t, C(), t.classesWithoutHost, !0),
    t.stylesWithoutHost != null &&
      mE(t) &&
      _u(n, t, C(), t.stylesWithoutHost, !1),
    sa
  );
}
function od(e, t, n, r) {
  return (Ao(e, t, n, r), sa(), od);
}
var IM = (e, t, n, r, o, i) => (bo(!0), Nl(r, o, fE()));
function CM(e, t, n, r, o) {
  let i = t.consts,
    s = gt(i, r),
    a = No(t, e, 8, "ng-container", s);
  s !== null && mu(a, s, !0);
  let c = gt(i, o);
  return (
    il() && Gl(t, n, a, c, Fl),
    (a.mergedAttrs = ir(a.mergedAttrs, a.attrs)),
    t.queries !== null && t.queries.elementStart(t, a),
    a
  );
}
function ly(e, t, n) {
  let r = C(),
    o = Q(),
    i = e + Y,
    s = o.firstCreatePass ? CM(i, o, r, t, n) : o.data[i];
  Qe(s, !0);
  let a = MM(o, r, s, e);
  return (
    (r[i] = a),
    Co() && Qs(o, r, a, s),
    Ht(a, r),
    Ls(s) && (Ws(o, r, s), Ml(o, s, r)),
    n != null && Pl(r, s),
    ly
  );
}
function dy() {
  let e = we(),
    t = Q();
  return (
    sl() ? al() : ((e = e.parent), Qe(e, !1)),
    t.firstCreatePass && (fl(t, e), nl(e) && t.queries.elementEnd(e)),
    dy
  );
}
function bM(e, t, n) {
  return (ly(e, t, n), dy(), bM);
}
var MM = (e, t, n, r) => (bo(!0), Wg(t[U], ""));
function P1() {
  return C();
}
function SM(e, t, n) {
  let r = C(),
    o = Cn();
  if (ve(r, o, t)) {
    let i = Q(),
      s = Us();
    Zs(i, s, r, e, t, r[U], n, !0);
  }
  return SM;
}
var cn = void 0;
function TM(e) {
  let t = Math.floor(Math.abs(e)),
    n = e.toString().replace(/^[^.]*\.?/, "").length;
  return t === 1 && n === 0 ? 1 : 5;
}
var _M = [
    "en",
    [["a", "p"], ["AM", "PM"], cn],
    [["AM", "PM"], cn, cn],
    [
      ["S", "M", "T", "W", "T", "F", "S"],
      ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"],
      [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
      ],
      ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"],
    ],
    cn,
    [
      ["J", "F", "M", "A", "M", "J", "J", "A", "S", "O", "N", "D"],
      [
        "Jan",
        "Feb",
        "Mar",
        "Apr",
        "May",
        "Jun",
        "Jul",
        "Aug",
        "Sep",
        "Oct",
        "Nov",
        "Dec",
      ],
      [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
      ],
    ],
    cn,
    [
      ["B", "A"],
      ["BC", "AD"],
      ["Before Christ", "Anno Domini"],
    ],
    0,
    [6, 0],
    ["M/d/yy", "MMM d, y", "MMMM d, y", "EEEE, MMMM d, y"],
    ["h:mm a", "h:mm:ss a", "h:mm:ss a z", "h:mm:ss a zzzz"],
    ["{1}, {0}", cn, "{1} 'at' {0}", cn],
    [".", ",", ";", "%", "+", "-", "E", "\xD7", "\u2030", "\u221E", "NaN", ":"],
    ["#,##0.###", "#,##0%", "\xA4#,##0.00", "#E0"],
    "USD",
    "$",
    "US Dollar",
    {},
    "ltr",
    TM,
  ],
  Sc = {};
function Oo(e) {
  let t = xM(e),
    n = qh(t);
  if (n) return n;
  let r = t.split("-")[0];
  if (((n = qh(r)), n)) return n;
  if (r === "en") return _M;
  throw new v(701, !1);
}
function fy(e) {
  return Oo(e)[qt.PluralCase];
}
function qh(e) {
  return (
    e in Sc ||
      (Sc[e] =
        ft.ng &&
        ft.ng.common &&
        ft.ng.common.locales &&
        ft.ng.common.locales[e]),
    Sc[e]
  );
}
var qt = (function (e) {
  return (
    (e[(e.LocaleId = 0)] = "LocaleId"),
    (e[(e.DayPeriodsFormat = 1)] = "DayPeriodsFormat"),
    (e[(e.DayPeriodsStandalone = 2)] = "DayPeriodsStandalone"),
    (e[(e.DaysFormat = 3)] = "DaysFormat"),
    (e[(e.DaysStandalone = 4)] = "DaysStandalone"),
    (e[(e.MonthsFormat = 5)] = "MonthsFormat"),
    (e[(e.MonthsStandalone = 6)] = "MonthsStandalone"),
    (e[(e.Eras = 7)] = "Eras"),
    (e[(e.FirstDayOfWeek = 8)] = "FirstDayOfWeek"),
    (e[(e.WeekendRange = 9)] = "WeekendRange"),
    (e[(e.DateFormat = 10)] = "DateFormat"),
    (e[(e.TimeFormat = 11)] = "TimeFormat"),
    (e[(e.DateTimeFormat = 12)] = "DateTimeFormat"),
    (e[(e.NumberSymbols = 13)] = "NumberSymbols"),
    (e[(e.NumberFormats = 14)] = "NumberFormats"),
    (e[(e.CurrencyCode = 15)] = "CurrencyCode"),
    (e[(e.CurrencySymbol = 16)] = "CurrencySymbol"),
    (e[(e.CurrencyName = 17)] = "CurrencyName"),
    (e[(e.Currencies = 18)] = "Currencies"),
    (e[(e.Directionality = 19)] = "Directionality"),
    (e[(e.PluralCase = 20)] = "PluralCase"),
    (e[(e.ExtraData = 21)] = "ExtraData"),
    e
  );
})(qt || {});
function xM(e) {
  return e.toLowerCase().replace(/_/g, "-");
}
var NM = ["zero", "one", "two", "few", "many"];
function RM(e, t) {
  let n = fy(t)(parseInt(e, 10)),
    r = NM[n];
  return r !== void 0 ? r : "other";
}
var Ts = "en-US",
  AM = "USD",
  hy = { marker: "element" },
  py = { marker: "ICU" },
  dt = (function (e) {
    return (
      (e[(e.SHIFT = 2)] = "SHIFT"),
      (e[(e.APPEND_EAGERLY = 1)] = "APPEND_EAGERLY"),
      (e[(e.COMMENT = 2)] = "COMMENT"),
      e
    );
  })(dt || {}),
  gy = Ts;
function OM(e) {
  typeof e == "string" && (gy = e.toLowerCase().replace(/_/g, "-"));
}
function kM() {
  return gy;
}
var mo = 0,
  ro = 0;
function PM(e) {
  (e && (mo = mo | (1 << Math.min(ro, 31))), ro++);
}
function FM(e, t, n) {
  if (ro > 0) {
    let r = e.data[n],
      o = Array.isArray(r) ? r : r.update,
      i = zp() - ro - 1;
    yy(e, t, o, i, mo);
  }
  ((mo = 0), (ro = 0));
}
function LM(e, t, n) {
  let r = e[U];
  switch (n) {
    case Node.COMMENT_NODE:
      return Wg(r, t);
    case Node.TEXT_NODE:
      return qg(r, t);
    case Node.ELEMENT_NODE:
      return Nl(r, t, null);
  }
}
var Ji = (e, t, n, r) => (bo(!0), LM(e, n, r));
function jM(e, t, n, r) {
  let o = e[U];
  for (let i = 0; i < t.length; i++) {
    let s = t[i++],
      a = t[i],
      c = (s & dt.COMMENT) === dt.COMMENT,
      u = (s & dt.APPEND_EAGERLY) === dt.APPEND_EAGERLY,
      l = s >>> dt.SHIFT,
      d = e[l],
      h = !1;
    (d === null &&
      ((d = e[l] = Ji(e, l, a, c ? Node.COMMENT_NODE : Node.TEXT_NODE)),
      (h = Co())),
      u && n !== null && h && vn(o, n, d, r, !1));
  }
}
function my(e, t, n, r) {
  let o = n[U],
    i = null,
    s;
  for (let a = 0; a < t.length; a++) {
    let c = t[a];
    if (typeof c == "string") {
      let u = t[++a];
      n[u] === null && (n[u] = Ji(n, u, c, Node.TEXT_NODE));
    } else if (typeof c == "number")
      switch (c & 1) {
        case 0:
          let u = AC(c);
          i === null && ((i = u), (s = o.parentNode(r)));
          let l, d;
          if (
            (u === i ? ((l = r), (d = s)) : ((l = null), (d = Pe(n[u]))),
            d !== null)
          ) {
            let g = OC(c),
              y = n[g];
            vn(o, d, y, l, !1);
            let w = no(e, g);
            if (w !== null && typeof w == "object") {
              let L = Xs(w, n);
              L !== null && my(e, w.create[L], n, n[w.anchorIdx]);
            }
          }
          break;
        case 1:
          let h = c >>> 1,
            f = t[++a],
            p = t[++a];
          Ll(o, js(h, n), null, null, f, p, null);
          break;
        default:
      }
    else
      switch (c) {
        case py:
          let u = t[++a],
            l = t[++a];
          if (n[l] === null) {
            let f = (n[l] = Ji(n, l, u, Node.COMMENT_NODE));
            Ht(f, n);
          }
          break;
        case hy:
          let d = t[++a],
            h = t[++a];
          if (n[h] === null) {
            let f = (n[h] = Ji(n, h, d, Node.ELEMENT_NODE));
            Ht(f, n);
          }
          break;
        default:
      }
  }
}
function yy(e, t, n, r, o) {
  for (let i = 0; i < n.length; i++) {
    let s = n[i],
      a = n[++i];
    if (s & o) {
      let c = "";
      for (let u = i + 1; u <= i + a; u++) {
        let l = n[u];
        if (typeof l == "string") c += l;
        else if (typeof l == "number")
          if (l < 0) c += Ut(t[r - l]);
          else {
            let d = l >>> 2;
            switch (l & 3) {
              case 1:
                let h = n[++u],
                  f = n[++u],
                  p = e.data[d];
                typeof p == "string"
                  ? Ll(t[U], t[d], null, p, h, c, f)
                  : Zs(e, p, t, h, c, t[U], f, !1);
                break;
              case 0:
                let g = t[d];
                g !== null && Gg(t[U], g, c);
                break;
              case 2:
                VM(e, no(e, d), t, c);
                break;
              case 3:
                Gh(e, no(e, d), r, t);
                break;
            }
          }
      }
    } else {
      let c = n[i + 1];
      if (c > 0 && (c & 3) === 3) {
        let u = c >>> 2,
          l = no(e, u);
        t[l.currentCaseLViewIndex] < 0 && Gh(e, l, r, t);
      }
    }
    i += a;
  }
}
function Gh(e, t, n, r) {
  let o = r[t.currentCaseLViewIndex];
  if (o !== null) {
    let i = mo;
    (o < 0 && ((o = r[t.currentCaseLViewIndex] = ~o), (i = -1)),
      yy(e, r, t.update[o], n, i));
  }
}
function VM(e, t, n, r) {
  let o = BM(t, r);
  if (
    Xs(t, n) !== o &&
    (vy(e, t, n),
    (n[t.currentCaseLViewIndex] = o === null ? null : ~o),
    o !== null)
  ) {
    let s = n[t.anchorIdx];
    (s && my(e, t.create[o], n, s), BC(n, t.anchorIdx, o));
  }
}
function vy(e, t, n) {
  let r = Xs(t, n);
  if (r !== null) {
    let o = t.remove[r];
    for (let i = 0; i < o.length; i++) {
      let s = o[i];
      if (s > 0) {
        let a = js(s, n);
        a !== null && Yg(n[U], a);
      } else vy(e, no(e, ~s), n);
    }
  }
}
function BM(e, t) {
  let n = e.cases.indexOf(t);
  if (n === -1)
    switch (e.type) {
      case 1: {
        let r = RM(t, kM());
        ((n = e.cases.indexOf(r)),
          n === -1 && r !== "other" && (n = e.cases.indexOf("other")));
        break;
      }
      case 0: {
        n = e.cases.indexOf("other");
        break;
      }
    }
  return n === -1 ? null : n;
}
var _s = /�(\d+):?\d*�/gi;
var UM = /�(\d+)�/,
  Dy = /^\s*(�\d+:?\d*�)\s*,\s*(select|plural)\s*,/,
  oo = "\uFFFD",
  $M = /�\/?\*(\d+:\d+)�/gi,
  HM = /�(\/?[#*]\d+):?\d*�/gi,
  zM = /\uE500/g;
function qM(e) {
  return e.replace(zM, " ");
}
function GM(e, t, n, r, o, i) {
  let s = co(),
    a = [],
    c = [],
    u = [[]],
    l = [[]];
  o = ZM(o, i);
  let d = qM(o).split(HM);
  for (let h = 0; h < d.length; h++) {
    let f = d[h];
    if ((h & 1) === 0) {
      let p = Pu(f);
      for (let g = 0; g < p.length; g++) {
        let y = p[g];
        if ((g & 1) === 0) {
          let w = y;
          w !== "" && WM(l[0], e, s, u[0], a, c, n, w);
        } else {
          let w = y;
          if (typeof w != "object")
            throw new Error(
              `Unable to parse ICU expression in "${o}" message.`,
            );
          let H = wy(e, s, u[0], n, a, "", !0).index;
          Iy(l[0], e, n, c, t, w, H);
        }
      }
    } else {
      let p = f.charCodeAt(0) === 47,
        g = f.charCodeAt(p ? 1 : 0),
        y = Y + Number.parseInt(f.substring(p ? 2 : 1));
      if (p) (u.shift(), l.shift(), Qe(co(), !1));
      else {
        let w = RC(e, u[0], y);
        (u.unshift([]), Qe(w, !0));
        let L = { kind: 2, index: y, children: [], type: g === 35 ? 0 : 1 };
        (l[0].push(L), l.unshift(L.children));
      }
    }
  }
  e.data[r] = { create: a, update: c, ast: l[0], parentTNodeIndex: t };
}
function wy(e, t, n, r, o, i, s) {
  let a = To(e, r, 1, null),
    c = a << dt.SHIFT,
    u = co();
  (t === u && (u = null),
    u === null && (c |= dt.APPEND_EAGERLY),
    s && ((c |= dt.COMMENT), JI(FC)),
    o.push(c, i === null ? "" : i));
  let l = ql(e, a, s ? 32 : 1, i === null ? "" : i, null);
  Cm(n, l);
  let d = l.index;
  return (Qe(l, !1), u !== null && t !== u && NC(u, d), l);
}
function WM(e, t, n, r, o, i, s, a) {
  let c = a.match(_s),
    l = wy(t, n, r, s, o, c ? null : a, !1).index;
  (c && Xi(i, a, l, null, 0, null), e.push({ kind: 0, index: l }));
}
function Xi(e, t, n, r, o, i) {
  let s = e.length,
    a = s + 1;
  e.push(null, null);
  let c = s + 2,
    u = t.split(_s),
    l = 0;
  for (let d = 0; d < u.length; d++) {
    let h = u[d];
    if (d & 1) {
      let f = o + parseInt(h, 10);
      (e.push(-1 - f), (l = l | Ey(f)));
    } else h !== "" && e.push(h);
  }
  return (
    e.push((n << 2) | (r ? 1 : 0)),
    r && e.push(r, i),
    (e[s] = l),
    (e[a] = e.length - c),
    l
  );
}
function Ey(e) {
  return 1 << Math.min(e, 31);
}
function Wh(e) {
  let t,
    n = "",
    r = 0,
    o = !1,
    i;
  for (; (t = $M.exec(e)) !== null; )
    o
      ? t[0] === `${oo}/*${i}${oo}` && ((r = t.index), (o = !1))
      : ((n += e.substring(r, t.index + t[0].length)), (i = t[1]), (o = !0));
  return ((n += e.slice(r)), n);
}
function ZM(e, t) {
  if (PC(t)) return Wh(e);
  {
    let n = e.indexOf(`:${t}${oo}`) + 2 + t.toString().length,
      r = e.search(new RegExp(`${oo}\\/\\*\\d+:${t}${oo}`));
    return Wh(e.substring(n, r));
  }
}
function Iy(e, t, n, r, o, i, s) {
  let a = 0,
    c = {
      type: i.type,
      currentCaseLViewIndex: To(t, n, 1, null),
      anchorIdx: s,
      cases: [],
      create: [],
      remove: [],
      update: [],
    };
  (JM(r, i, s), xC(t, s, c));
  let u = i.values,
    l = [];
  for (let d = 0; d < u.length; d++) {
    let h = u[d],
      f = [];
    for (let g = 0; g < h.length; g++) {
      let y = h[g];
      if (typeof y != "string") {
        let w = f.push(y) - 1;
        h[g] = `<!--\uFFFD${w}\uFFFD-->`;
      }
    }
    let p = [];
    (l.push(p), (a = QM(p, t, c, n, r, o, i.cases[d], h.join(""), f) | a));
  }
  (a && XM(r, a, s),
    e.push({
      kind: 3,
      index: s,
      cases: l,
      currentCaseLViewIndex: c.currentCaseLViewIndex,
    }));
}
function YM(e) {
  let t = [],
    n = [],
    r = 1,
    o = 0;
  e = e.replace(Dy, function (s, a, c) {
    return (
      c === "select" ? (r = 0) : (r = 1),
      (o = parseInt(a.slice(1), 10)),
      ""
    );
  });
  let i = Pu(e);
  for (let s = 0; s < i.length; ) {
    let a = i[s++].trim();
    (r === 1 && (a = a.replace(/\s*(?:=)?(\w+)\s*/, "$1")),
      a.length && t.push(a));
    let c = Pu(i[s++]);
    t.length > n.length && n.push(c);
  }
  return { type: r, mainBinding: o, cases: t, values: n };
}
function Pu(e) {
  if (!e) return [];
  let t = 0,
    n = [],
    r = [],
    o = /[{}]/g;
  o.lastIndex = 0;
  let i;
  for (; (i = o.exec(e)); ) {
    let a = i.index;
    if (i[0] == "}") {
      if ((n.pop(), n.length == 0)) {
        let c = e.substring(t, a);
        (Dy.test(c) ? r.push(YM(c)) : r.push(c), (t = a + 1));
      }
    } else {
      if (n.length == 0) {
        let c = e.substring(t, a);
        (r.push(c), (t = a + 1));
      }
      n.push("{");
    }
  }
  let s = e.substring(t);
  return (r.push(s), r);
}
function QM(e, t, n, r, o, i, s, a, c) {
  let u = [],
    l = [],
    d = [];
  (n.cases.push(s), n.create.push(u), n.remove.push(l), n.update.push(d));
  let f = Pg(vl()).getInertBodyElement(a),
    p = cu(f) || f;
  return p ? Cy(e, t, n, r, o, u, l, d, p, i, c, 0) : 0;
}
function Cy(e, t, n, r, o, i, s, a, c, u, l, d) {
  let h = 0,
    f = c.firstChild;
  for (; f; ) {
    let p = To(t, r, 1, null);
    switch (f.nodeType) {
      case Node.ELEMENT_NODE:
        let g = f,
          y = g.tagName.toLowerCase();
        if (su.hasOwnProperty(y)) {
          (Tc(i, hy, y, u, p), (t.data[p] = y));
          let Ae = g.attributes;
          for (let $r = 0; $r < Ae.length; $r++) {
            let Kt = Ae.item($r),
              wf = Kt.name.toLowerCase();
            !!Kt.value.match(_s)
              ? Vg.hasOwnProperty(wf) &&
                (Sl[wf]
                  ? Xi(a, Kt.value, p, Kt.name, 0, Mo)
                  : Xi(a, Kt.value, p, Kt.name, 0, null))
              : eS(i, p, Kt);
          }
          let st = { kind: 1, index: p, children: [] };
          (e.push(st),
            (h = Cy(st.children, t, n, r, o, i, s, a, f, p, l, d + 1) | h),
            Zh(s, p, d));
        }
        break;
      case Node.TEXT_NODE:
        let w = f.textContent || "",
          L = w.match(_s);
        (Tc(i, null, L ? "" : w, u, p),
          Zh(s, p, d),
          L && (h = Xi(a, w, p, null, 0, null) | h),
          e.push({ kind: 0, index: p }));
        break;
      case Node.COMMENT_NODE:
        let H = UM.exec(f.textContent || "");
        if (H) {
          let Ae = parseInt(H[1], 10),
            st = l[Ae];
          (Tc(i, py, "", u, p), Iy(e, t, r, o, u, st, p), KM(s, p, d));
        }
        break;
    }
    f = f.nextSibling;
  }
  return h;
}
function Zh(e, t, n) {
  n === 0 && e.push(t);
}
function KM(e, t, n) {
  n === 0 && (e.push(~t), e.push(t));
}
function JM(e, t, n) {
  e.push(Ey(t.mainBinding), 2, -1 - t.mainBinding, (n << 2) | 2);
}
function XM(e, t, n) {
  e.push(t, 1, (n << 2) | 3);
}
function Tc(e, t, n, r, o) {
  (t !== null && e.push(t), e.push(n, o, kC(0, r, o)));
}
function eS(e, t, n) {
  e.push((t << 1) | 1, n.name, n.value);
}
var Yh = 0,
  tS = /\[(�.+?�?)\]/,
  nS = /\[(�.+?�?)\]|(�\/?\*\d+:\d+�)/g,
  rS = /({\s*)(VAR_(PLURAL|SELECT)(_\d+)?)(\s*,)/g,
  oS = /{([A-Z0-9_]+)}/g,
  iS = /�I18N_EXP_(ICU(_\d+)?)�/g,
  sS = /\/\*/,
  aS = /\d+\:(\d+)/;
function cS(e, t = {}) {
  let n = e;
  if (tS.test(e)) {
    let r = {},
      o = [Yh];
    n = n.replace(nS, (i, s, a) => {
      let c = s || a,
        u = r[c] || [];
      if (
        (u.length ||
          (c.split("|").forEach((g) => {
            let y = g.match(aS),
              w = y ? parseInt(y[1], 10) : Yh,
              L = sS.test(g);
            u.push([w, L, g]);
          }),
          (r[c] = u)),
        !u.length)
      )
        throw new Error(`i18n postprocess: unmatched placeholder - ${c}`);
      let l = o[o.length - 1],
        d = 0;
      for (let g = 0; g < u.length; g++)
        if (u[g][0] === l) {
          d = g;
          break;
        }
      let [h, f, p] = u[d];
      return (f ? o.pop() : l !== h && o.push(h), u.splice(d, 1), p);
    });
  }
  return (
    Object.keys(t).length &&
      ((n = n.replace(rS, (r, o, i, s, a, c) =>
        t.hasOwnProperty(i) ? `${o}${t[i]}${c}` : r,
      )),
      (n = n.replace(oS, (r, o) => (t.hasOwnProperty(o) ? t[o] : r))),
      (n = n.replace(iS, (r, o) => {
        if (t.hasOwnProperty(o)) {
          let i = t[o];
          if (!i.length)
            throw new Error(
              `i18n postprocess: unmatched ICU - ${r} with key: ${o}`,
            );
          return i.shift();
        }
        return r;
      }))),
    n
  );
}
function uS(e, t, n = -1) {
  let r = Q(),
    o = C(),
    i = Y + e,
    s = gt(r.consts, t),
    a = co();
  if (
    (r.firstCreatePass && GM(r, a === null ? 0 : a.index, o, i, s, n),
    r.type === 2)
  ) {
    let h = o[be];
    h[_] |= 32;
  } else o[_] |= 32;
  let c = r.data[i],
    u = a === o[De] ? null : a,
    l = nm(r, u, o),
    d = a && a.type & 8 ? o[a.index] : null;
  (jC(o, i, a, n), jM(o, c.create, l, d), qp(!0));
}
function lS() {
  qp(!1);
}
function F1(e, t, n) {
  (uS(e, t, n), lS());
}
function dS(e) {
  let t = C();
  return (PM(ve(t, Cn(), e)), dS);
}
function L1(e) {
  FM(Q(), C(), e + Y);
}
function j1(e, t = {}) {
  return cS(e, t);
}
function Qh(e, t, n, r) {
  return function o(i) {
    if (i === Function) return r;
    let s = yr(e) ? Ye(e.index, t) : t;
    zl(s, 5);
    let a = Kh(t, n, r, i),
      c = o.__ngNextListenerFn__;
    for (; c; ) ((a = Kh(t, n, c, i) && a), (c = c.__ngNextListenerFn__));
    return a;
  };
}
function Kh(e, t, n, r) {
  let o = A(null);
  try {
    return ($(6, t, n), n(r) !== !1);
  } catch (i) {
    return (fS(e, i), !1);
  } finally {
    ($(7, t, n), A(o));
  }
}
function fS(e, t) {
  let n = e[rr],
    r = n ? n.get(Ue, null) : null;
  r && r.handleError(t);
}
function Jh(e, t, n, r, o, i, s, a, c) {
  let u = n[r],
    d = t.data[r].outputs[o],
    f = u[d].subscribe(s),
    p = a.length;
  (a.push(s, f), c && c.push(i, e.index, p, -(p + 1)));
}
var hS = (e, t, n) => {};
function id(e, t, n, r) {
  let o = C(),
    i = Q(),
    s = we();
  return (gS(i, o, o[U], s, e, t, r), id);
}
function pS(e, t, n, r) {
  let o = e.cleanup;
  if (o != null)
    for (let i = 0; i < o.length - 1; i += 2) {
      let s = o[i];
      if (s === n && o[i + 1] === r) {
        let a = t[ss],
          c = o[i + 2];
        return a.length > c ? a[c] : null;
      }
      typeof s == "string" && (i += 2);
    }
  return null;
}
function gS(e, t, n, r, o, i, s) {
  let a = Ls(r),
    u = e.firstCreatePass ? Bp(e) : null,
    l = t[ne],
    d = Vp(t),
    h = !0;
  if (r.type & 3 || s) {
    let f = Xe(r, t),
      p = s ? s(f) : f,
      g = d.length,
      y = s ? (L) => s(Pe(L[r.index])) : r.index,
      w = null;
    if ((!s && a && (w = pS(e, t, o, r.index)), w !== null)) {
      let L = w.__ngLastListenerFn__ || w;
      ((L.__ngNextListenerFn__ = i), (w.__ngLastListenerFn__ = i), (h = !1));
    } else {
      ((i = Qh(r, t, l, i)), hS(p, o, i));
      let L = n.listen(p, o, i);
      (d.push(i, L), u && u.push(o, y, g, g + 1));
    }
  } else i = Qh(r, t, l, i);
  if (h) {
    let f = r.outputs?.[o],
      p = r.hostDirectiveOutputs?.[o];
    if (p && p.length)
      for (let g = 0; g < p.length; g += 2) {
        let y = p[g],
          w = p[g + 1];
        Jh(r, e, t, y, w, o, i, d, u);
      }
    if (f && f.length) for (let g of f) Jh(r, e, t, g, o, o, i, d, u);
  }
}
function V1(e = 1) {
  return lE(e);
}
function mS(e, t) {
  let n = null,
    r = SI(e);
  for (let o = 0; o < t.length; o++) {
    let i = t[o];
    if (i === "*") {
      n = o;
      continue;
    }
    if (r === null ? zg(e, i, !0) : xI(r, i)) return o;
  }
  return n;
}
function by(e) {
  let t = C()[be][De];
  if (!t.projection) {
    let n = e ? e.length : 1,
      r = (t.projection = _w(n, null)),
      o = r.slice(),
      i = t.child;
    for (; i !== null; ) {
      if (i.type !== 128) {
        let s = e ? mS(i, e) : 0;
        s !== null &&
          (o[s] ? (o[s].projectionNext = i) : (r[s] = i), (o[s] = i));
      }
      i = i.next;
    }
  }
}
function My(e, t = 0, n, r, o, i) {
  let s = C(),
    a = Q(),
    c = r ? e + 1 : null;
  c !== null && Cs(s, a, c, r, o, i, null, n);
  let u = No(a, Y + e, 16, null, n || null);
  (u.projection === null && (u.projection = t), al());
  let d = !s[nr] || Up();
  s[be][De].projection[u.projection] === null && c !== null
    ? yS(s, a, c)
    : d && !Ul(u) && oC(a, s, u);
}
function yS(e, t, n) {
  let r = Y + n,
    o = t.data[r],
    i = e[r],
    s = cr(i, o.tView.ssrId),
    a = _o(e, o, void 0, { dehydratedView: s });
  xo(i, a, 0, ar(o, s));
}
function vS(e, t, n) {
  return (Sy(e, "", t, "", n), vS);
}
function Sy(e, t, n, r, o) {
  let i = C(),
    s = td(i, t, n, r);
  if (s !== Se) {
    let a = Q(),
      c = Us();
    Zs(a, c, i, e, s, i[U], o, !1);
  }
  return Sy;
}
function B1(e, t, n) {
  Pm(e, t, n);
}
function DS(e) {
  let t = C(),
    n = Q(),
    r = ul();
  Bs(r + 1);
  let o = Zl(n, r);
  if (e.dirty && Ww(t) === ((o.metadata.flags & 2) === 2)) {
    if (o.matches === null) e.reset([]);
    else {
      let i = Lm(t, r);
      (e.reset(i, gg), e.notifyOnChanges());
    }
    return !0;
  }
  return !1;
}
function wS() {
  return Wl(C(), ul());
}
function U1(e, t, n, r, o) {
  Bm(t, Db(e, n, r, o));
}
function $1(e, t, n, r) {
  Bm(e, Pm(t, n, r));
}
function H1(e = 1) {
  Bs(ul() + e);
}
function ES(e, t, n, r) {
  (n >= e.data.length && ((e.data[n] = null), (e.blueprint[n] = null)),
    (t[n] = r));
}
function z1(e) {
  let t = rE();
  return Io(t, Y + e);
}
function Ty(e, t = "") {
  let n = C(),
    r = Q(),
    o = e + Y,
    i = r.firstCreatePass ? No(r, o, 1, t, null) : r.data[o],
    s = IS(r, n, i, t, e);
  ((n[o] = s), Co() && Qs(r, n, s, i), Qe(i, !1));
}
var IS = (e, t, n, r, o) => (bo(!0), qg(t[U], r));
function sd(e) {
  return (_y("", e, ""), sd);
}
function _y(e, t, n) {
  let r = C(),
    o = td(r, e, t, n);
  return (o !== Se && xy(r, Et(), o), _y);
}
function CS(e, t, n, r, o) {
  let i = C(),
    s = Kb(i, e, t, n, r, o);
  return (s !== Se && xy(i, Et(), s), CS);
}
function xy(e, t, n) {
  let r = js(t, e);
  Gg(e[U], r, n);
}
function bS(e, t, n) {
  let r = Q();
  if (r.firstCreatePass) {
    let o = Ze(e);
    (Fu(n, r.data, r.blueprint, o, !0), Fu(t, r.data, r.blueprint, o, !1));
  }
}
function Fu(e, t, n, r, o) {
  if (((e = ae(e)), Array.isArray(e)))
    for (let i = 0; i < e.length; i++) Fu(e[i], t, n, r, o);
  else {
    let i = Q(),
      s = C(),
      a = we(),
      c = tr(e) ? e : ae(e.provide),
      u = Tp(e),
      l = a.providerIndexes & 1048575,
      d = a.directiveStart,
      h = a.providerIndexes >> 20;
    if (tr(e) || !e.multi) {
      let f = new yn(u, o, q),
        p = xc(c, t, o ? l : l + h, d);
      p === -1
        ? (zc(hs(a, s), i, c),
          _c(i, e, t.length),
          t.push(c),
          a.directiveStart++,
          a.directiveEnd++,
          o && (a.providerIndexes += 1048576),
          n.push(f),
          s.push(f))
        : ((n[p] = f), (s[p] = f));
    } else {
      let f = xc(c, t, l + h, d),
        p = xc(c, t, l, l + h),
        g = f >= 0 && n[f],
        y = p >= 0 && n[p];
      if ((o && !y) || (!o && !g)) {
        zc(hs(a, s), i, c);
        let w = TS(o ? SS : MS, n.length, o, r, u);
        (!o && y && (n[p].providerFactory = w),
          _c(i, e, t.length, 0),
          t.push(c),
          a.directiveStart++,
          a.directiveEnd++,
          o && (a.providerIndexes += 1048576),
          n.push(w),
          s.push(w));
      } else {
        let w = Ny(n[o ? p : f], u, !o && r);
        _c(i, e, f > -1 ? f : p, w);
      }
      !o && r && y && n[p].componentProviders++;
    }
  }
}
function _c(e, t, n, r) {
  let o = tr(t),
    i = Fw(t);
  if (o || i) {
    let c = (i ? ae(t.useClass) : t).prototype.ngOnDestroy;
    if (c) {
      let u = e.destroyHooks || (e.destroyHooks = []);
      if (!o && t.multi) {
        let l = u.indexOf(n);
        l === -1 ? u.push(n, [r, c]) : u[l + 1].push(r, c);
      } else u.push(n, c);
    }
  }
}
function Ny(e, t, n) {
  return (n && e.componentProviders++, e.multi.push(t) - 1);
}
function xc(e, t, n, r) {
  for (let o = n; o < r; o++) if (t[o] === e) return o;
  return -1;
}
function MS(e, t, n, r) {
  return Lu(this.multi, []);
}
function SS(e, t, n, r) {
  let o = this.multi,
    i;
  if (this.providerFactory) {
    let s = this.providerFactory.componentProviders,
      a = uo(n, n[N], this.providerFactory.index, r);
    ((i = a.slice(0, s)), Lu(o, i));
    for (let c = s; c < a.length; c++) i.push(a[c]);
  } else ((i = []), Lu(o, i));
  return i;
}
function Lu(e, t) {
  for (let n = 0; n < e.length; n++) {
    let r = e[n];
    t.push(r());
  }
  return t;
}
function TS(e, t, n, r, o) {
  let i = new yn(e, n, q);
  return (
    (i.multi = []),
    (i.index = t),
    (i.componentProviders = 0),
    Ny(i, o, r && !n),
    i
  );
}
function q1(e, t = []) {
  return (n) => {
    n.providersResolver = (r, o) => bS(r, o ? o(e) : e, t);
  };
}
function G1(e, t, n) {
  let r = et() + e,
    o = C();
  return o[r] === Se ? Sn(o, r, n ? t.call(n) : t()) : Wm(o, r);
}
function W1(e, t, n, r) {
  return Ry(C(), et(), e, t, n, r);
}
function Z1(e, t, n, r, o) {
  return _S(C(), et(), e, t, n, r, o);
}
function Y1(e, t, n, r, o, i) {
  return Ay(C(), et(), e, t, n, r, o, i);
}
function Q1(e, t, n, r, o, i, s) {
  return Oy(C(), et(), e, t, n, r, o, i, s);
}
function K1(e, t, n, r, o, i, s, a) {
  let c = et() + e,
    u = C(),
    l = Zm(u, c, n, r, o, i);
  return ve(u, c + 4, s) || l
    ? Sn(u, c + 5, a ? t.call(a, n, r, o, i, s) : t(n, r, o, i, s))
    : Wm(u, c + 5);
}
function ko(e, t) {
  let n = e[t];
  return n === Se ? void 0 : n;
}
function Ry(e, t, n, r, o, i) {
  let s = t + n;
  return ve(e, s, o) ? Sn(e, s + 1, i ? r.call(i, o) : r(o)) : ko(e, s + 1);
}
function _S(e, t, n, r, o, i, s) {
  let a = t + n;
  return po(e, a, o, i)
    ? Sn(e, a + 2, s ? r.call(s, o, i) : r(o, i))
    : ko(e, a + 2);
}
function Ay(e, t, n, r, o, i, s, a) {
  let c = t + n;
  return $b(e, c, o, i, s)
    ? Sn(e, c + 3, a ? r.call(a, o, i, s) : r(o, i, s))
    : ko(e, c + 3);
}
function Oy(e, t, n, r, o, i, s, a, c) {
  let u = t + n;
  return Zm(e, u, o, i, s, a)
    ? Sn(e, u + 4, c ? r.call(c, o, i, s, a) : r(o, i, s, a))
    : ko(e, u + 4);
}
function xS(e, t, n, r, o, i) {
  let s = t + n,
    a = !1;
  for (let c = 0; c < o.length; c++) ve(e, s++, o[c]) && (a = !0);
  return a ? Sn(e, s, r.apply(i, o)) : ko(e, s);
}
function J1(e, t) {
  let n = Q(),
    r,
    o = e + Y;
  n.firstCreatePass
    ? ((r = NS(t, n.pipeRegistry)),
      (n.data[o] = r),
      r.onDestroy && (n.destroyHooks ??= []).push(o, r.onDestroy))
    : (r = n.data[o]);
  let i = r.factory || (r.factory = dn(r.type, !0)),
    s,
    a = Ie(q);
  try {
    let c = fs(!1),
      u = i();
    return (fs(c), ES(n, C(), o, u), u);
  } finally {
    Ie(a);
  }
}
function NS(e, t) {
  if (t)
    for (let n = t.length - 1; n >= 0; n--) {
      let r = t[n];
      if (e === r.name) return r;
    }
}
function X1(e, t, n) {
  let r = e + Y,
    o = C(),
    i = Io(o, r);
  return aa(o, r) ? Ry(o, et(), t, i.transform, n, i) : i.transform(n);
}
function eL(e, t, n, r, o) {
  let i = e + Y,
    s = C(),
    a = Io(s, i);
  return aa(s, i)
    ? Ay(s, et(), t, a.transform, n, r, o, a)
    : a.transform(n, r, o);
}
function tL(e, t, n, r, o, i) {
  let s = e + Y,
    a = C(),
    c = Io(a, s);
  return aa(a, s)
    ? Oy(a, et(), t, c.transform, n, r, o, i, c)
    : c.transform(n, r, o, i);
}
function nL(e, t, n) {
  let r = e + Y,
    o = C(),
    i = Io(o, r);
  return aa(o, r) ? xS(o, et(), t, i.transform, n, i) : i.transform.apply(i, n);
}
function aa(e, t) {
  return e[N].data[t].pure;
}
function rL(e, t) {
  return Js(e, t);
}
var fr = class {
    full;
    major;
    minor;
    patch;
    constructor(t) {
      this.full = t;
      let n = t.split(".");
      ((this.major = n[0]),
        (this.minor = n[1]),
        (this.patch = n.slice(2).join(".")));
    }
  },
  oL = new fr("19.2.4"),
  ju = class {
    ngModuleFactory;
    componentFactories;
    constructor(t, n) {
      ((this.ngModuleFactory = t), (this.componentFactories = n));
    }
  },
  ca = (() => {
    class e {
      compileModuleSync(n) {
        return new Cu(n);
      }
      compileModuleAsync(n) {
        return Promise.resolve(this.compileModuleSync(n));
      }
      compileModuleAndAllComponentsSync(n) {
        let r = this.compileModuleSync(n),
          o = Ip(n),
          i = $g(o.declarations).reduce((s, a) => {
            let c = $t(a);
            return (c && s.push(new wn(c)), s);
          }, []);
        return new ju(r, i);
      }
      compileModuleAndAllComponentsAsync(n) {
        return Promise.resolve(this.compileModuleAndAllComponentsSync(n));
      }
      clearCache() {}
      clearCacheFor(n) {}
      getModuleId(n) {}
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵprov = E({ token: e, factory: e.ɵfac, providedIn: "root" });
    }
    return e;
  })();
var RS = (() => {
    class e {
      zone = m(J);
      changeDetectionScheduler = m(mt);
      applicationRef = m(vt);
      _onMicrotaskEmptySubscription;
      initialize() {
        this._onMicrotaskEmptySubscription ||
          (this._onMicrotaskEmptySubscription =
            this.zone.onMicrotaskEmpty.subscribe({
              next: () => {
                this.changeDetectionScheduler.runningTick ||
                  this.zone.run(() => {
                    this.applicationRef.tick();
                  });
              },
            }));
      }
      ngOnDestroy() {
        this._onMicrotaskEmptySubscription?.unsubscribe();
      }
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵprov = E({ token: e, factory: e.ɵfac, providedIn: "root" });
    }
    return e;
  })(),
  AS = new I("", { factory: () => !1 });
function ky({
  ngZoneFactory: e,
  ignoreChangesOutsideZone: t,
  scheduleInRootZone: n,
}) {
  return (
    (e ??= () => new J(j(D({}, Py()), { scheduleInRootZone: n }))),
    [
      { provide: J, useFactory: e },
      {
        provide: er,
        multi: !0,
        useFactory: () => {
          let r = m(RS, { optional: !0 });
          return () => r.initialize();
        },
      },
      {
        provide: er,
        multi: !0,
        useFactory: () => {
          let r = m(OS);
          return () => {
            r.initialize();
          };
        },
      },
      t === !0 ? { provide: dg, useValue: !0 } : [],
      { provide: fg, useValue: n ?? lg },
    ]
  );
}
function iL(e) {
  let t = e?.ignoreChangesOutsideZone,
    n = e?.scheduleInRootZone,
    r = ky({
      ngZoneFactory: () => {
        let o = Py(e);
        return (
          (o.scheduleInRootZone = n),
          o.shouldCoalesceEventChangeDetection && zt("NgZone_CoalesceEvent"),
          new J(o)
        );
      },
      ignoreChangesOutsideZone: t,
      scheduleInRootZone: n,
    });
  return pr([{ provide: AS, useValue: !0 }, { provide: $s, useValue: !1 }, r]);
}
function Py(e) {
  return {
    enableLongStackTrace: !1,
    shouldCoalesceEventChangeDetection: e?.eventCoalescing ?? !1,
    shouldCoalesceRunChangeDetection: e?.runCoalescing ?? !1,
  };
}
var OS = (() => {
  class e {
    subscription = new K();
    initialized = !1;
    zone = m(J);
    pendingTasks = m(Ct);
    initialize() {
      if (this.initialized) return;
      this.initialized = !0;
      let n = null;
      (!this.zone.isStable &&
        !this.zone.hasPendingMacrotasks &&
        !this.zone.hasPendingMicrotasks &&
        (n = this.pendingTasks.add()),
        this.zone.runOutsideAngular(() => {
          this.subscription.add(
            this.zone.onStable.subscribe(() => {
              (J.assertNotInAngularZone(),
                queueMicrotask(() => {
                  n !== null &&
                    !this.zone.hasPendingMacrotasks &&
                    !this.zone.hasPendingMicrotasks &&
                    (this.pendingTasks.remove(n), (n = null));
                }));
            }),
          );
        }),
        this.subscription.add(
          this.zone.onUnstable.subscribe(() => {
            (J.assertInAngularZone(), (n ??= this.pendingTasks.add()));
          }),
        ));
    }
    ngOnDestroy() {
      this.subscription.unsubscribe();
    }
    static ɵfac = function (r) {
      return new (r || e)();
    };
    static ɵprov = E({ token: e, factory: e.ɵfac, providedIn: "root" });
  }
  return e;
})();
var kS = (() => {
  class e {
    appRef = m(vt);
    taskService = m(Ct);
    ngZone = m(J);
    zonelessEnabled = m($s);
    tracing = m(Mn, { optional: !0 });
    disableScheduling = m(dg, { optional: !0 }) ?? !1;
    zoneIsDefined = typeof Zone < "u" && !!Zone.root.run;
    schedulerTickApplyArgs = [{ data: { __scheduler_tick__: !0 } }];
    subscriptions = new K();
    angularZoneId = this.zoneIsDefined ? this.ngZone._inner?.get(gs) : null;
    scheduleInRootZone =
      !this.zonelessEnabled &&
      this.zoneIsDefined &&
      (m(fg, { optional: !0 }) ?? !1);
    cancelScheduledCallback = null;
    useMicrotaskScheduler = !1;
    runningTick = !1;
    pendingRenderTaskId = null;
    constructor() {
      (this.subscriptions.add(
        this.appRef.afterTick.subscribe(() => {
          this.runningTick || this.cleanup();
        }),
      ),
        this.subscriptions.add(
          this.ngZone.onUnstable.subscribe(() => {
            this.runningTick || this.cleanup();
          }),
        ),
        (this.disableScheduling ||=
          !this.zonelessEnabled &&
          (this.ngZone instanceof Zc || !this.zoneIsDefined)));
    }
    notify(n) {
      if (!this.zonelessEnabled && n === 5) return;
      let r = !1;
      switch (n) {
        case 0: {
          this.appRef.dirtyFlags |= 2;
          break;
        }
        case 3:
        case 2:
        case 4:
        case 5:
        case 1: {
          this.appRef.dirtyFlags |= 4;
          break;
        }
        case 6: {
          ((this.appRef.dirtyFlags |= 2), (r = !0));
          break;
        }
        case 12: {
          ((this.appRef.dirtyFlags |= 16), (r = !0));
          break;
        }
        case 13: {
          ((this.appRef.dirtyFlags |= 2), (r = !0));
          break;
        }
        case 11: {
          r = !0;
          break;
        }
        case 9:
        case 8:
        case 7:
        case 10:
        default:
          this.appRef.dirtyFlags |= 8;
      }
      if (
        ((this.appRef.tracingSnapshot =
          this.tracing?.snapshot(this.appRef.tracingSnapshot) ?? null),
        !this.shouldScheduleTick(r))
      )
        return;
      let o = this.useMicrotaskScheduler ? mh : hg;
      ((this.pendingRenderTaskId = this.taskService.add()),
        this.scheduleInRootZone
          ? (this.cancelScheduledCallback = Zone.root.run(() =>
              o(() => this.tick()),
            ))
          : (this.cancelScheduledCallback = this.ngZone.runOutsideAngular(() =>
              o(() => this.tick()),
            )));
    }
    shouldScheduleTick(n) {
      return !(
        (this.disableScheduling && !n) ||
        this.appRef.destroyed ||
        this.pendingRenderTaskId !== null ||
        this.runningTick ||
        this.appRef._runningTick ||
        (!this.zonelessEnabled &&
          this.zoneIsDefined &&
          Zone.current.get(gs + this.angularZoneId))
      );
    }
    tick() {
      if (this.runningTick || this.appRef.destroyed) return;
      if (this.appRef.dirtyFlags === 0) {
        this.cleanup();
        return;
      }
      !this.zonelessEnabled &&
        this.appRef.dirtyFlags & 7 &&
        (this.appRef.dirtyFlags |= 1);
      let n = this.taskService.add();
      try {
        this.ngZone.run(
          () => {
            ((this.runningTick = !0), this.appRef._tick());
          },
          void 0,
          this.schedulerTickApplyArgs,
        );
      } catch (r) {
        throw (this.taskService.remove(n), r);
      } finally {
        this.cleanup();
      }
      ((this.useMicrotaskScheduler = !0),
        mh(() => {
          ((this.useMicrotaskScheduler = !1), this.taskService.remove(n));
        }));
    }
    ngOnDestroy() {
      (this.subscriptions.unsubscribe(), this.cleanup());
    }
    cleanup() {
      if (
        ((this.runningTick = !1),
        this.cancelScheduledCallback?.(),
        (this.cancelScheduledCallback = null),
        this.pendingRenderTaskId !== null)
      ) {
        let n = this.pendingRenderTaskId;
        ((this.pendingRenderTaskId = null), this.taskService.remove(n));
      }
    }
    static ɵfac = function (r) {
      return new (r || e)();
    };
    static ɵprov = E({ token: e, factory: e.ɵfac, providedIn: "root" });
  }
  return e;
})();
function PS() {
  return (typeof $localize < "u" && $localize.locale) || Ts;
}
var ua = new I("", {
    providedIn: "root",
    factory: () => m(ua, P.Optional | P.SkipSelf) || PS(),
  }),
  Fy = new I("", { providedIn: "root", factory: () => AM });
var Vu = new I(""),
  FS = new I("");
function Jr(e) {
  return !e.moduleRef;
}
function LS(e) {
  let t = Jr(e) ? e.r3Injector : e.moduleRef.injector,
    n = t.get(J);
  return n.run(() => {
    Jr(e)
      ? e.r3Injector.resolveInjectorInitializers()
      : e.moduleRef.resolveInjectorInitializers();
    let r = t.get(Ue, null),
      o;
    if (
      (n.runOutsideAngular(() => {
        o = n.onError.subscribe({
          next: (i) => {
            r.handleError(i);
          },
        });
      }),
      Jr(e))
    ) {
      let i = () => t.destroy(),
        s = e.platformInjector.get(Vu);
      (s.add(i),
        t.onDestroy(() => {
          (o.unsubscribe(), s.delete(i));
        }));
    } else {
      let i = () => e.moduleRef.destroy(),
        s = e.platformInjector.get(Vu);
      (s.add(i),
        e.moduleRef.onDestroy(() => {
          (Ki(e.allPlatformModules, e.moduleRef), o.unsubscribe(), s.delete(i));
        }));
    }
    return VS(r, n, () => {
      let i = t.get(Km);
      return (
        i.runInitializers(),
        i.donePromise.then(() => {
          let s = t.get(ua, Ts);
          if ((OM(s || Ts), !t.get(FS, !0)))
            return Jr(e)
              ? t.get(vt)
              : (e.allPlatformModules.push(e.moduleRef), e.moduleRef);
          if (Jr(e)) {
            let c = t.get(vt);
            return (
              e.rootComponent !== void 0 && c.bootstrap(e.rootComponent),
              c
            );
          } else return (jS(e.moduleRef, e.allPlatformModules), e.moduleRef);
        })
      );
    });
  });
}
function jS(e, t) {
  let n = e.injector.get(vt);
  if (e._bootstrapComponents.length > 0)
    e._bootstrapComponents.forEach((r) => n.bootstrap(r));
  else if (e.instance.ngDoBootstrap) e.instance.ngDoBootstrap(n);
  else throw new v(-403, !1);
  t.push(e);
}
function VS(e, t, n) {
  try {
    let r = n();
    return Cr(r)
      ? r.catch((o) => {
          throw (t.runOutsideAngular(() => e.handleError(o)), o);
        })
      : r;
  } catch (r) {
    throw (t.runOutsideAngular(() => e.handleError(r)), r);
  }
}
var es = null;
function BS(e = [], t) {
  return ce.create({
    name: t,
    providers: [
      { provide: ks, useValue: "platform" },
      { provide: Vu, useValue: new Set([() => (es = null)]) },
      ...e,
    ],
  });
}
function US(e = []) {
  if (es) return es;
  let t = BS(e);
  return ((es = t), Wb(), $S(t), t);
}
function $S(e) {
  let t = e.get(wl, null);
  Me(e, () => {
    t?.forEach((n) => n());
  });
}
var br = (() => {
  class e {
    static __NG_ELEMENT_ID__ = HS;
  }
  return e;
})();
function HS(e) {
  return zS(we(), C(), (e & 16) === 16);
}
function zS(e, t, n) {
  if (yr(e) && !n) {
    let r = Ye(e.index, t);
    return new ho(r, r);
  } else if (e.type & 175) {
    let r = t[be];
    return new ho(r, t);
  }
  return null;
}
var Bu = class {
    constructor() {}
    supports(t) {
      return Gm(t);
    }
    create(t) {
      return new Uu(t);
    }
  },
  qS = (e, t) => t,
  Uu = class {
    length = 0;
    collection;
    _linkedRecords = null;
    _unlinkedRecords = null;
    _previousItHead = null;
    _itHead = null;
    _itTail = null;
    _additionsHead = null;
    _additionsTail = null;
    _movesHead = null;
    _movesTail = null;
    _removalsHead = null;
    _removalsTail = null;
    _identityChangesHead = null;
    _identityChangesTail = null;
    _trackByFn;
    constructor(t) {
      this._trackByFn = t || qS;
    }
    forEachItem(t) {
      let n;
      for (n = this._itHead; n !== null; n = n._next) t(n);
    }
    forEachOperation(t) {
      let n = this._itHead,
        r = this._removalsHead,
        o = 0,
        i = null;
      for (; n || r; ) {
        let s = !r || (n && n.currentIndex < Xh(r, o, i)) ? n : r,
          a = Xh(s, o, i),
          c = s.currentIndex;
        if (s === r) (o--, (r = r._nextRemoved));
        else if (((n = n._next), s.previousIndex == null)) o++;
        else {
          i || (i = []);
          let u = a - o,
            l = c - o;
          if (u != l) {
            for (let h = 0; h < u; h++) {
              let f = h < i.length ? i[h] : (i[h] = 0),
                p = f + h;
              l <= p && p < u && (i[h] = f + 1);
            }
            let d = s.previousIndex;
            i[d] = l - u;
          }
        }
        a !== c && t(s, a, c);
      }
    }
    forEachPreviousItem(t) {
      let n;
      for (n = this._previousItHead; n !== null; n = n._nextPrevious) t(n);
    }
    forEachAddedItem(t) {
      let n;
      for (n = this._additionsHead; n !== null; n = n._nextAdded) t(n);
    }
    forEachMovedItem(t) {
      let n;
      for (n = this._movesHead; n !== null; n = n._nextMoved) t(n);
    }
    forEachRemovedItem(t) {
      let n;
      for (n = this._removalsHead; n !== null; n = n._nextRemoved) t(n);
    }
    forEachIdentityChange(t) {
      let n;
      for (n = this._identityChangesHead; n !== null; n = n._nextIdentityChange)
        t(n);
    }
    diff(t) {
      if ((t == null && (t = []), !Gm(t))) throw new v(900, !1);
      return this.check(t) ? this : null;
    }
    onDestroy() {}
    check(t) {
      this._reset();
      let n = this._itHead,
        r = !1,
        o,
        i,
        s;
      if (Array.isArray(t)) {
        this.length = t.length;
        for (let a = 0; a < this.length; a++)
          ((i = t[a]),
            (s = this._trackByFn(a, i)),
            n === null || !Object.is(n.trackById, s)
              ? ((n = this._mismatch(n, i, s, a)), (r = !0))
              : (r && (n = this._verifyReinsertion(n, i, s, a)),
                Object.is(n.item, i) || this._addIdentityChange(n, i)),
            (n = n._next));
      } else
        ((o = 0),
          Ub(t, (a) => {
            ((s = this._trackByFn(o, a)),
              n === null || !Object.is(n.trackById, s)
                ? ((n = this._mismatch(n, a, s, o)), (r = !0))
                : (r && (n = this._verifyReinsertion(n, a, s, o)),
                  Object.is(n.item, a) || this._addIdentityChange(n, a)),
              (n = n._next),
              o++);
          }),
          (this.length = o));
      return (this._truncate(n), (this.collection = t), this.isDirty);
    }
    get isDirty() {
      return (
        this._additionsHead !== null ||
        this._movesHead !== null ||
        this._removalsHead !== null ||
        this._identityChangesHead !== null
      );
    }
    _reset() {
      if (this.isDirty) {
        let t;
        for (t = this._previousItHead = this._itHead; t !== null; t = t._next)
          t._nextPrevious = t._next;
        for (t = this._additionsHead; t !== null; t = t._nextAdded)
          t.previousIndex = t.currentIndex;
        for (
          this._additionsHead = this._additionsTail = null, t = this._movesHead;
          t !== null;
          t = t._nextMoved
        )
          t.previousIndex = t.currentIndex;
        ((this._movesHead = this._movesTail = null),
          (this._removalsHead = this._removalsTail = null),
          (this._identityChangesHead = this._identityChangesTail = null));
      }
    }
    _mismatch(t, n, r, o) {
      let i;
      return (
        t === null ? (i = this._itTail) : ((i = t._prev), this._remove(t)),
        (t =
          this._unlinkedRecords === null
            ? null
            : this._unlinkedRecords.get(r, null)),
        t !== null
          ? (Object.is(t.item, n) || this._addIdentityChange(t, n),
            this._reinsertAfter(t, i, o))
          : ((t =
              this._linkedRecords === null
                ? null
                : this._linkedRecords.get(r, o)),
            t !== null
              ? (Object.is(t.item, n) || this._addIdentityChange(t, n),
                this._moveAfter(t, i, o))
              : (t = this._addAfter(new $u(n, r), i, o))),
        t
      );
    }
    _verifyReinsertion(t, n, r, o) {
      let i =
        this._unlinkedRecords === null
          ? null
          : this._unlinkedRecords.get(r, null);
      return (
        i !== null
          ? (t = this._reinsertAfter(i, t._prev, o))
          : t.currentIndex != o &&
            ((t.currentIndex = o), this._addToMoves(t, o)),
        t
      );
    }
    _truncate(t) {
      for (; t !== null; ) {
        let n = t._next;
        (this._addToRemovals(this._unlink(t)), (t = n));
      }
      (this._unlinkedRecords !== null && this._unlinkedRecords.clear(),
        this._additionsTail !== null && (this._additionsTail._nextAdded = null),
        this._movesTail !== null && (this._movesTail._nextMoved = null),
        this._itTail !== null && (this._itTail._next = null),
        this._removalsTail !== null && (this._removalsTail._nextRemoved = null),
        this._identityChangesTail !== null &&
          (this._identityChangesTail._nextIdentityChange = null));
    }
    _reinsertAfter(t, n, r) {
      this._unlinkedRecords !== null && this._unlinkedRecords.remove(t);
      let o = t._prevRemoved,
        i = t._nextRemoved;
      return (
        o === null ? (this._removalsHead = i) : (o._nextRemoved = i),
        i === null ? (this._removalsTail = o) : (i._prevRemoved = o),
        this._insertAfter(t, n, r),
        this._addToMoves(t, r),
        t
      );
    }
    _moveAfter(t, n, r) {
      return (
        this._unlink(t),
        this._insertAfter(t, n, r),
        this._addToMoves(t, r),
        t
      );
    }
    _addAfter(t, n, r) {
      return (
        this._insertAfter(t, n, r),
        this._additionsTail === null
          ? (this._additionsTail = this._additionsHead = t)
          : (this._additionsTail = this._additionsTail._nextAdded = t),
        t
      );
    }
    _insertAfter(t, n, r) {
      let o = n === null ? this._itHead : n._next;
      return (
        (t._next = o),
        (t._prev = n),
        o === null ? (this._itTail = t) : (o._prev = t),
        n === null ? (this._itHead = t) : (n._next = t),
        this._linkedRecords === null && (this._linkedRecords = new xs()),
        this._linkedRecords.put(t),
        (t.currentIndex = r),
        t
      );
    }
    _remove(t) {
      return this._addToRemovals(this._unlink(t));
    }
    _unlink(t) {
      this._linkedRecords !== null && this._linkedRecords.remove(t);
      let n = t._prev,
        r = t._next;
      return (
        n === null ? (this._itHead = r) : (n._next = r),
        r === null ? (this._itTail = n) : (r._prev = n),
        t
      );
    }
    _addToMoves(t, n) {
      return (
        t.previousIndex === n ||
          (this._movesTail === null
            ? (this._movesTail = this._movesHead = t)
            : (this._movesTail = this._movesTail._nextMoved = t)),
        t
      );
    }
    _addToRemovals(t) {
      return (
        this._unlinkedRecords === null && (this._unlinkedRecords = new xs()),
        this._unlinkedRecords.put(t),
        (t.currentIndex = null),
        (t._nextRemoved = null),
        this._removalsTail === null
          ? ((this._removalsTail = this._removalsHead = t),
            (t._prevRemoved = null))
          : ((t._prevRemoved = this._removalsTail),
            (this._removalsTail = this._removalsTail._nextRemoved = t)),
        t
      );
    }
    _addIdentityChange(t, n) {
      return (
        (t.item = n),
        this._identityChangesTail === null
          ? (this._identityChangesTail = this._identityChangesHead = t)
          : (this._identityChangesTail =
              this._identityChangesTail._nextIdentityChange =
                t),
        t
      );
    }
  },
  $u = class {
    item;
    trackById;
    currentIndex = null;
    previousIndex = null;
    _nextPrevious = null;
    _prev = null;
    _next = null;
    _prevDup = null;
    _nextDup = null;
    _prevRemoved = null;
    _nextRemoved = null;
    _nextAdded = null;
    _nextMoved = null;
    _nextIdentityChange = null;
    constructor(t, n) {
      ((this.item = t), (this.trackById = n));
    }
  },
  Hu = class {
    _head = null;
    _tail = null;
    add(t) {
      this._head === null
        ? ((this._head = this._tail = t),
          (t._nextDup = null),
          (t._prevDup = null))
        : ((this._tail._nextDup = t),
          (t._prevDup = this._tail),
          (t._nextDup = null),
          (this._tail = t));
    }
    get(t, n) {
      let r;
      for (r = this._head; r !== null; r = r._nextDup)
        if ((n === null || n <= r.currentIndex) && Object.is(r.trackById, t))
          return r;
      return null;
    }
    remove(t) {
      let n = t._prevDup,
        r = t._nextDup;
      return (
        n === null ? (this._head = r) : (n._nextDup = r),
        r === null ? (this._tail = n) : (r._prevDup = n),
        this._head === null
      );
    }
  },
  xs = class {
    map = new Map();
    put(t) {
      let n = t.trackById,
        r = this.map.get(n);
      (r || ((r = new Hu()), this.map.set(n, r)), r.add(t));
    }
    get(t, n) {
      let r = t,
        o = this.map.get(r);
      return o ? o.get(t, n) : null;
    }
    remove(t) {
      let n = t.trackById;
      return (this.map.get(n).remove(t) && this.map.delete(n), t);
    }
    get isEmpty() {
      return this.map.size === 0;
    }
    clear() {
      this.map.clear();
    }
  };
function Xh(e, t, n) {
  let r = e.previousIndex;
  if (r === null) return r;
  let o = 0;
  return (n && r < n.length && (o = n[r]), r + t + o);
}
var zu = class {
    constructor() {}
    supports(t) {
      return t instanceof Map || Kl(t);
    }
    create() {
      return new qu();
    }
  },
  qu = class {
    _records = new Map();
    _mapHead = null;
    _appendAfter = null;
    _previousMapHead = null;
    _changesHead = null;
    _changesTail = null;
    _additionsHead = null;
    _additionsTail = null;
    _removalsHead = null;
    _removalsTail = null;
    get isDirty() {
      return (
        this._additionsHead !== null ||
        this._changesHead !== null ||
        this._removalsHead !== null
      );
    }
    forEachItem(t) {
      let n;
      for (n = this._mapHead; n !== null; n = n._next) t(n);
    }
    forEachPreviousItem(t) {
      let n;
      for (n = this._previousMapHead; n !== null; n = n._nextPrevious) t(n);
    }
    forEachChangedItem(t) {
      let n;
      for (n = this._changesHead; n !== null; n = n._nextChanged) t(n);
    }
    forEachAddedItem(t) {
      let n;
      for (n = this._additionsHead; n !== null; n = n._nextAdded) t(n);
    }
    forEachRemovedItem(t) {
      let n;
      for (n = this._removalsHead; n !== null; n = n._nextRemoved) t(n);
    }
    diff(t) {
      if (!t) t = new Map();
      else if (!(t instanceof Map || Kl(t))) throw new v(900, !1);
      return this.check(t) ? this : null;
    }
    onDestroy() {}
    check(t) {
      this._reset();
      let n = this._mapHead;
      if (
        ((this._appendAfter = null),
        this._forEach(t, (r, o) => {
          if (n && n.key === o)
            (this._maybeAddToChanges(n, r),
              (this._appendAfter = n),
              (n = n._next));
          else {
            let i = this._getOrCreateRecordForKey(o, r);
            n = this._insertBeforeOrAppend(n, i);
          }
        }),
        n)
      ) {
        (n._prev && (n._prev._next = null), (this._removalsHead = n));
        for (let r = n; r !== null; r = r._nextRemoved)
          (r === this._mapHead && (this._mapHead = null),
            this._records.delete(r.key),
            (r._nextRemoved = r._next),
            (r.previousValue = r.currentValue),
            (r.currentValue = null),
            (r._prev = null),
            (r._next = null));
      }
      return (
        this._changesTail && (this._changesTail._nextChanged = null),
        this._additionsTail && (this._additionsTail._nextAdded = null),
        this.isDirty
      );
    }
    _insertBeforeOrAppend(t, n) {
      if (t) {
        let r = t._prev;
        return (
          (n._next = t),
          (n._prev = r),
          (t._prev = n),
          r && (r._next = n),
          t === this._mapHead && (this._mapHead = n),
          (this._appendAfter = t),
          t
        );
      }
      return (
        this._appendAfter
          ? ((this._appendAfter._next = n), (n._prev = this._appendAfter))
          : (this._mapHead = n),
        (this._appendAfter = n),
        null
      );
    }
    _getOrCreateRecordForKey(t, n) {
      if (this._records.has(t)) {
        let o = this._records.get(t);
        this._maybeAddToChanges(o, n);
        let i = o._prev,
          s = o._next;
        return (
          i && (i._next = s),
          s && (s._prev = i),
          (o._next = null),
          (o._prev = null),
          o
        );
      }
      let r = new Gu(t);
      return (
        this._records.set(t, r),
        (r.currentValue = n),
        this._addToAdditions(r),
        r
      );
    }
    _reset() {
      if (this.isDirty) {
        let t;
        for (
          this._previousMapHead = this._mapHead, t = this._previousMapHead;
          t !== null;
          t = t._next
        )
          t._nextPrevious = t._next;
        for (t = this._changesHead; t !== null; t = t._nextChanged)
          t.previousValue = t.currentValue;
        for (t = this._additionsHead; t != null; t = t._nextAdded)
          t.previousValue = t.currentValue;
        ((this._changesHead = this._changesTail = null),
          (this._additionsHead = this._additionsTail = null),
          (this._removalsHead = null));
      }
    }
    _maybeAddToChanges(t, n) {
      Object.is(n, t.currentValue) ||
        ((t.previousValue = t.currentValue),
        (t.currentValue = n),
        this._addToChanges(t));
    }
    _addToAdditions(t) {
      this._additionsHead === null
        ? (this._additionsHead = this._additionsTail = t)
        : ((this._additionsTail._nextAdded = t), (this._additionsTail = t));
    }
    _addToChanges(t) {
      this._changesHead === null
        ? (this._changesHead = this._changesTail = t)
        : ((this._changesTail._nextChanged = t), (this._changesTail = t));
    }
    _forEach(t, n) {
      t instanceof Map
        ? t.forEach(n)
        : Object.keys(t).forEach((r) => n(t[r], r));
    }
  },
  Gu = class {
    key;
    previousValue = null;
    currentValue = null;
    _nextPrevious = null;
    _next = null;
    _prev = null;
    _nextAdded = null;
    _nextRemoved = null;
    _nextChanged = null;
    constructor(t) {
      this.key = t;
    }
  };
function ep() {
  return new ad([new Bu()]);
}
var ad = (() => {
  class e {
    factories;
    static ɵprov = E({ token: e, providedIn: "root", factory: ep });
    constructor(n) {
      this.factories = n;
    }
    static create(n, r) {
      if (r != null) {
        let o = r.factories.slice();
        n = n.concat(o);
      }
      return new e(n);
    }
    static extend(n) {
      return {
        provide: e,
        useFactory: (r) => e.create(n, r || ep()),
        deps: [[e, new vp(), new Xu()]],
      };
    }
    find(n) {
      let r = this.factories.find((o) => o.supports(n));
      if (r != null) return r;
      throw new v(901, !1);
    }
  }
  return e;
})();
function tp() {
  return new cd([new zu()]);
}
var cd = (() => {
  class e {
    static ɵprov = E({ token: e, providedIn: "root", factory: tp });
    factories;
    constructor(n) {
      this.factories = n;
    }
    static create(n, r) {
      if (r) {
        let o = r.factories.slice();
        n = n.concat(o);
      }
      return new e(n);
    }
    static extend(n) {
      return {
        provide: e,
        useFactory: (r) => e.create(n, r || tp()),
        deps: [[e, new vp(), new Xu()]],
      };
    }
    find(n) {
      let r = this.factories.find((o) => o.supports(n));
      if (r) return r;
      throw new v(901, !1);
    }
  }
  return e;
})();
function Ly(e) {
  $(8);
  try {
    let { rootComponent: t, appProviders: n, platformProviders: r } = e,
      o = US(r),
      i = [ky({}), { provide: mt, useExisting: kS }, ...(n || [])],
      s = new Is({
        providers: i,
        parent: o,
        debugName: "",
        runEnvironmentInitializers: !1,
      });
    return LS({
      r3Injector: s.injector,
      platformInjector: o,
      rootComponent: t,
    });
  } catch (t) {
    return Promise.reject(t);
  } finally {
    $(9);
  }
}
function Po(e) {
  return typeof e == "boolean" ? e : e != null && e !== "false";
}
function GS(e, t = NaN) {
  return !isNaN(parseFloat(e)) && !isNaN(Number(e)) ? Number(e) : t;
}
function Te(e) {
  return Qa(e);
}
function io(e, t) {
  return fi(e, t?.equal);
}
var Wu = class {
  [re];
  constructor(t) {
    this[re] = t;
  }
  destroy() {
    this[re].destroy();
  }
};
function jy(e, t) {
  !t?.injector && gr(jy);
  let n = t?.injector ?? m(ce),
    r = t?.manualCleanup !== !0 ? n.get(It) : null,
    o,
    i = n.get(zs, null, { optional: !0 }),
    s = n.get(mt);
  return (
    i !== null && !t?.forceRoot
      ? ((o = YS(i.view, s, e)),
        r instanceof ps && r._lView === i.view && (r = null))
      : (o = QS(e, n.get(Qm), s)),
    (o.injector = n),
    r !== null && (o.onDestroyFn = r.onDestroy(() => o.destroy())),
    new Wu(o)
  );
}
var Vy = j(D({}, Xt), {
    consumerIsAlwaysLive: !0,
    consumerAllowSignalWrites: !0,
    dirty: !0,
    hasRun: !1,
    cleanupFns: void 0,
    zone: null,
    kind: "effect",
    onDestroyFn: lo,
    run() {
      if (((this.dirty = !1), this.hasRun && !qr(this))) return;
      this.hasRun = !0;
      let e = (r) => (this.cleanupFns ??= []).push(r),
        t = en(this),
        n = us(!1);
      try {
        (this.maybeCleanup(), this.fn(e));
      } finally {
        (us(n), kn(this, t));
      }
    },
    maybeCleanup() {
      if (this.cleanupFns?.length)
        try {
          for (; this.cleanupFns.length; ) this.cleanupFns.pop()();
        } finally {
          this.cleanupFns = [];
        }
    },
  }),
  WS = j(D({}, Vy), {
    consumerMarkedDirty() {
      (this.scheduler.schedule(this), this.notifier.notify(12));
    },
    destroy() {
      (Gr(this),
        this.onDestroyFn(),
        this.maybeCleanup(),
        this.scheduler.remove(this));
    },
  }),
  ZS = j(D({}, Vy), {
    consumerMarkedDirty() {
      ((this.view[_] |= 8192), Dr(this.view), this.notifier.notify(13));
    },
    destroy() {
      (Gr(this),
        this.onDestroyFn(),
        this.maybeCleanup(),
        this.view[hn]?.delete(this));
    },
  });
function YS(e, t, n) {
  let r = Object.create(ZS);
  return (
    (r.view = e),
    (r.zone = typeof Zone < "u" ? Zone.current : null),
    (r.notifier = t),
    (r.fn = n),
    (e[hn] ??= new Set()),
    e[hn].add(r),
    r.consumerMarkedDirty(r),
    r
  );
}
function QS(e, t, n) {
  let r = Object.create(WS);
  return (
    (r.fn = e),
    (r.scheduler = t),
    (r.notifier = n),
    (r.zone = typeof Zone < "u" ? Zone.current : null),
    r.scheduler.schedule(r),
    r.notifier.notify(12),
    r
  );
}
var W = (function (e) {
    return (
      (e[(e.Idle = 0)] = "Idle"),
      (e[(e.Error = 1)] = "Error"),
      (e[(e.Loading = 2)] = "Loading"),
      (e[(e.Reloading = 3)] = "Reloading"),
      (e[(e.Resolved = 4)] = "Resolved"),
      (e[(e.Local = 5)] = "Local"),
      e
    );
  })(W || {}),
  KS = (e) => e;
function Zu(e, t) {
  if (typeof e == "function") {
    let n = pi(e, KS, t?.equal);
    return np(n);
  } else {
    let n = pi(e.source, e.computation, e.equal);
    return np(n);
  }
}
function np(e) {
  let t = e[re],
    n = e;
  return (
    (n.set = (r) => Za(t, r)),
    (n.update = (r) => Ya(t, r)),
    (n.asReadonly = Hs.bind(e)),
    n
  );
}
function JS(e) {
  e?.injector || gr(JS);
  let t = e.request ?? (() => null);
  return new Ns(
    t,
    eT(e),
    e.defaultValue,
    e.equal ? XS(e.equal) : void 0,
    e.injector ?? m(ce),
  );
}
var Yu = class {
    value;
    constructor(t) {
      ((this.value = t),
        (this.value.set = this.set.bind(this)),
        (this.value.update = this.update.bind(this)),
        (this.value.asReadonly = Hs));
    }
    update(t) {
      this.set(t(Te(this.value)));
    }
    isLoading = io(
      () => this.status() === W.Loading || this.status() === W.Reloading,
    );
    hasValue() {
      return this.value() !== void 0;
    }
    asReadonly() {
      return this;
    }
  },
  Ns = class extends Yu {
    loaderFn;
    defaultValue;
    equal;
    pendingTasks;
    state;
    extRequest;
    effectRef;
    pendingController;
    resolvePendingTask = void 0;
    destroyed = !1;
    constructor(t, n, r, o, i) {
      (super(
        io(
          () => {
            let s = this.state().stream?.();
            return s && Qu(s) ? s.value : this.defaultValue;
          },
          { equal: o },
        ),
      ),
        (this.loaderFn = n),
        (this.defaultValue = r),
        (this.equal = o),
        (this.extRequest = Zu({
          source: t,
          computation: (s) => ({ request: s, reload: 0 }),
        })),
        (this.state = Zu({
          source: this.extRequest,
          computation: (s, a) => {
            let c = s.request === void 0 ? W.Idle : W.Loading;
            return a
              ? {
                  extRequest: s,
                  status: c,
                  previousStatus: rp(a.value),
                  stream:
                    a.value.extRequest.request === s.request
                      ? a.value.stream
                      : void 0,
                }
              : {
                  extRequest: s,
                  status: c,
                  previousStatus: W.Idle,
                  stream: void 0,
                };
          },
        })),
        (this.effectRef = jy(this.loadEffect.bind(this), {
          injector: i,
          manualCleanup: !0,
        })),
        (this.pendingTasks = i.get(NE)),
        i.get(It).onDestroy(() => this.destroy()));
    }
    status = io(() => rp(this.state()));
    error = io(() => {
      let t = this.state().stream?.();
      return t && !Qu(t) ? t.error : void 0;
    });
    set(t) {
      if (this.destroyed) return;
      let n = Te(this.value),
        r = Te(this.state);
      (r.status === W.Local && (this.equal ? this.equal(n, t) : n === t)) ||
        (this.state.set({
          extRequest: r.extRequest,
          status: W.Local,
          previousStatus: W.Local,
          stream: sr({ value: t }),
        }),
        this.abortInProgressLoad());
    }
    reload() {
      let { status: t } = Te(this.state);
      return t === W.Idle || t === W.Loading
        ? !1
        : (this.extRequest.update(({ request: n, reload: r }) => ({
            request: n,
            reload: r + 1,
          })),
          !0);
    }
    destroy() {
      ((this.destroyed = !0),
        this.effectRef.destroy(),
        this.abortInProgressLoad(),
        this.state.set({
          extRequest: { request: void 0, reload: 0 },
          status: W.Idle,
          previousStatus: W.Idle,
          stream: void 0,
        }));
    }
    loadEffect() {
      return Rn(this, null, function* () {
        let t = this.extRequest(),
          { status: n, previousStatus: r } = Te(this.state);
        if (t.request === void 0) return;
        if (n !== W.Loading) return;
        this.abortInProgressLoad();
        let o = (this.resolvePendingTask = this.pendingTasks.add()),
          { signal: i } = (this.pendingController = new AbortController());
        try {
          let s = yield Te(() =>
            this.loaderFn({
              request: t.request,
              abortSignal: i,
              previous: { status: r },
            }),
          );
          if (i.aborted || Te(this.extRequest) !== t) return;
          this.state.set({
            extRequest: t,
            status: W.Resolved,
            previousStatus: W.Resolved,
            stream: s,
          });
        } catch (s) {
          if (i.aborted || Te(this.extRequest) !== t) return;
          this.state.set({
            extRequest: t,
            status: W.Resolved,
            previousStatus: W.Error,
            stream: sr({ error: s }),
          });
        } finally {
          (o?.(), (o = void 0));
        }
      });
    }
    abortInProgressLoad() {
      (Te(() => this.pendingController?.abort()),
        (this.pendingController = void 0),
        this.resolvePendingTask?.(),
        (this.resolvePendingTask = void 0));
    }
  };
function XS(e) {
  return (t, n) => (t === void 0 || n === void 0 ? t === n : e(t, n));
}
function eT(e) {
  return tT(e)
    ? e.stream
    : (t) =>
        Rn(this, null, function* () {
          try {
            return sr({ value: yield e.loader(t) });
          } catch (n) {
            return sr({ error: n });
          }
        });
}
function tT(e) {
  return !!e.stream;
}
function rp(e) {
  switch (e.status) {
    case W.Loading:
      return e.extRequest.reload === 0 ? W.Loading : W.Reloading;
    case W.Resolved:
      return Qu(Te(e.stream)) ? W.Resolved : W.Error;
    default:
      return e.status;
  }
}
function Qu(e) {
  return e.error === void 0;
}
var Nc = Symbol("NOT_SET"),
  By = new Set(),
  nT = j(D({}, Wr), {
    consumerIsAlwaysLive: !0,
    consumerAllowSignalWrites: !0,
    value: Nc,
    cleanup: null,
    consumerMarkedDirty() {
      if (this.sequence.impl.executing) {
        if (
          this.sequence.lastPhase === null ||
          this.sequence.lastPhase < this.phase
        )
          return;
        this.sequence.erroredOrDestroyed = !0;
      }
      this.sequence.scheduler.notify(7);
    },
    phaseFn(e) {
      if (((this.sequence.lastPhase = this.phase), !this.dirty))
        return this.signal;
      if (((this.dirty = !1), this.value !== Nc && !qr(this)))
        return this.signal;
      try {
        for (let o of this.cleanup ?? By) o();
      } finally {
        this.cleanup?.clear();
      }
      let t = [];
      (e !== void 0 && t.push(e), t.push(this.registerCleanupFn));
      let n = en(this),
        r;
      try {
        r = this.userFn.apply(null, t);
      } finally {
        kn(this, n);
      }
      return (
        (this.value === Nc || !this.equal(this.value, r)) &&
          ((this.value = r), this.version++),
        this.signal
      );
    },
  }),
  Ku = class extends ys {
    scheduler;
    lastPhase = null;
    nodes = [void 0, void 0, void 0, void 0];
    constructor(t, n, r, o, i, s = null) {
      (super(t, [void 0, void 0, void 0, void 0], r, !1, i, s),
        (this.scheduler = o));
      for (let a of bg) {
        let c = n[a];
        if (c === void 0) continue;
        let u = Object.create(nT);
        ((u.sequence = this),
          (u.phase = a),
          (u.userFn = c),
          (u.dirty = !0),
          (u.signal = () => (Ot(u), u.value)),
          (u.signal[re] = u),
          (u.registerCleanupFn = (l) => (u.cleanup ??= new Set()).add(l)),
          (this.nodes[a] = u),
          (this.hooks[a] = (l) => u.phaseFn(l)));
      }
    }
    afterRun() {
      (super.afterRun(), (this.lastPhase = null));
    }
    destroy() {
      super.destroy();
      for (let t of this.nodes) for (let n of t?.cleanup ?? By) n();
    }
  };
function rT(e, t) {
  !t?.injector && gr(rT);
  let n = t?.injector ?? m(ce),
    r = n.get(mt),
    o = n.get(Cl),
    i = n.get(Mn, null, { optional: !0 });
  o.impl ??= n.get(Mg);
  let s = e;
  typeof s == "function" && (s = { mixedReadWrite: e });
  let a = n.get(zs, null, { optional: !0 }),
    c = new Ku(
      o.impl,
      [s.earlyRead, s.write, s.mixedReadWrite, s.read],
      a?.view,
      r,
      n.get(It),
      i?.snapshot(null),
    );
  return (o.impl.register(c), c);
}
function sL(e, t) {
  let n = $t(e),
    r = t.elementInjector || Ps();
  return new wn(n).create(
    r,
    t.projectableNodes,
    t.hostElement,
    t.environmentInjector,
  );
}
function Uy(e) {
  let t = $t(e);
  if (!t) return null;
  let n = new wn(t);
  return {
    get selector() {
      return n.selector;
    },
    get type() {
      return n.componentType;
    },
    get inputs() {
      return n.inputs;
    },
    get outputs() {
      return n.outputs;
    },
    get ngContentSelectors() {
      return n.ngContentSelectors;
    },
    get isStandalone() {
      return t.standalone;
    },
    get isSignal() {
      return t.signals;
    },
  };
}
var Qy = null;
function Mr() {
  return Qy;
}
function Ky(e) {
  Qy ??= e;
}
var la = class {};
var ue = new I(""),
  yd = (() => {
    class e {
      historyGo(n) {
        throw new Error("");
      }
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵprov = E({
        token: e,
        factory: () => m(oT),
        providedIn: "platform",
      });
    }
    return e;
  })(),
  Jy = new I(""),
  oT = (() => {
    class e extends yd {
      _location;
      _history;
      _doc = m(ue);
      constructor() {
        (super(),
          (this._location = window.location),
          (this._history = window.history));
      }
      getBaseHrefFromDOM() {
        return Mr().getBaseHref(this._doc);
      }
      onPopState(n) {
        let r = Mr().getGlobalEventTarget(this._doc, "window");
        return (
          r.addEventListener("popstate", n, !1),
          () => r.removeEventListener("popstate", n)
        );
      }
      onHashChange(n) {
        let r = Mr().getGlobalEventTarget(this._doc, "window");
        return (
          r.addEventListener("hashchange", n, !1),
          () => r.removeEventListener("hashchange", n)
        );
      }
      get href() {
        return this._location.href;
      }
      get protocol() {
        return this._location.protocol;
      }
      get hostname() {
        return this._location.hostname;
      }
      get port() {
        return this._location.port;
      }
      get pathname() {
        return this._location.pathname;
      }
      get search() {
        return this._location.search;
      }
      get hash() {
        return this._location.hash;
      }
      set pathname(n) {
        this._location.pathname = n;
      }
      pushState(n, r, o) {
        this._history.pushState(n, r, o);
      }
      replaceState(n, r, o) {
        this._history.replaceState(n, r, o);
      }
      forward() {
        this._history.forward();
      }
      back() {
        this._history.back();
      }
      historyGo(n = 0) {
        this._history.go(n);
      }
      getState() {
        return this._history.state;
      }
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵprov = E({
        token: e,
        factory: () => new e(),
        providedIn: "platform",
      });
    }
    return e;
  })();
function vd(e, t) {
  return e
    ? t
      ? e.endsWith("/")
        ? t.startsWith("/")
          ? e + t.slice(1)
          : e + t
        : t.startsWith("/")
          ? e + t
          : `${e}/${t}`
      : e
    : t;
}
function $y(e) {
  let t = e.search(/#|\?|$/);
  return e[t - 1] === "/" ? e.slice(0, t - 1) + e.slice(t) : e;
}
function Tt(e) {
  return e && e[0] !== "?" ? `?${e}` : e;
}
var xt = (() => {
    class e {
      historyGo(n) {
        throw new Error("");
      }
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵprov = E({ token: e, factory: () => m(Dd), providedIn: "root" });
    }
    return e;
  })(),
  Xy = new I(""),
  Dd = (() => {
    class e extends xt {
      _platformLocation;
      _baseHref;
      _removeListenerFns = [];
      constructor(n, r) {
        (super(),
          (this._platformLocation = n),
          (this._baseHref =
            r ??
            this._platformLocation.getBaseHrefFromDOM() ??
            m(ue).location?.origin ??
            ""));
      }
      ngOnDestroy() {
        for (; this._removeListenerFns.length; )
          this._removeListenerFns.pop()();
      }
      onPopState(n) {
        this._removeListenerFns.push(
          this._platformLocation.onPopState(n),
          this._platformLocation.onHashChange(n),
        );
      }
      getBaseHref() {
        return this._baseHref;
      }
      prepareExternalUrl(n) {
        return vd(this._baseHref, n);
      }
      path(n = !1) {
        let r =
            this._platformLocation.pathname + Tt(this._platformLocation.search),
          o = this._platformLocation.hash;
        return o && n ? `${r}${o}` : r;
      }
      pushState(n, r, o, i) {
        let s = this.prepareExternalUrl(o + Tt(i));
        this._platformLocation.pushState(n, r, s);
      }
      replaceState(n, r, o, i) {
        let s = this.prepareExternalUrl(o + Tt(i));
        this._platformLocation.replaceState(n, r, s);
      }
      forward() {
        this._platformLocation.forward();
      }
      back() {
        this._platformLocation.back();
      }
      getState() {
        return this._platformLocation.getState();
      }
      historyGo(n = 0) {
        this._platformLocation.historyGo?.(n);
      }
      static ɵfac = function (r) {
        return new (r || e)(b(yd), b(Xy, 8));
      };
      static ɵprov = E({ token: e, factory: e.ɵfac, providedIn: "root" });
    }
    return e;
  })(),
  ev = (() => {
    class e extends xt {
      _platformLocation;
      _baseHref = "";
      _removeListenerFns = [];
      constructor(n, r) {
        (super(),
          (this._platformLocation = n),
          r != null && (this._baseHref = r));
      }
      ngOnDestroy() {
        for (; this._removeListenerFns.length; )
          this._removeListenerFns.pop()();
      }
      onPopState(n) {
        this._removeListenerFns.push(
          this._platformLocation.onPopState(n),
          this._platformLocation.onHashChange(n),
        );
      }
      getBaseHref() {
        return this._baseHref;
      }
      path(n = !1) {
        let r = this._platformLocation.hash ?? "#";
        return r.length > 0 ? r.substring(1) : r;
      }
      prepareExternalUrl(n) {
        let r = vd(this._baseHref, n);
        return r.length > 0 ? "#" + r : r;
      }
      pushState(n, r, o, i) {
        let s =
          this.prepareExternalUrl(o + Tt(i)) || this._platformLocation.pathname;
        this._platformLocation.pushState(n, r, s);
      }
      replaceState(n, r, o, i) {
        let s =
          this.prepareExternalUrl(o + Tt(i)) || this._platformLocation.pathname;
        this._platformLocation.replaceState(n, r, s);
      }
      forward() {
        this._platformLocation.forward();
      }
      back() {
        this._platformLocation.back();
      }
      getState() {
        return this._platformLocation.getState();
      }
      historyGo(n = 0) {
        this._platformLocation.historyGo?.(n);
      }
      static ɵfac = function (r) {
        return new (r || e)(b(yd), b(Xy, 8));
      };
      static ɵprov = E({ token: e, factory: e.ɵfac });
    }
    return e;
  })(),
  Tr = (() => {
    class e {
      _subject = new X();
      _basePath;
      _locationStrategy;
      _urlChangeListeners = [];
      _urlChangeSubscription = null;
      constructor(n) {
        this._locationStrategy = n;
        let r = this._locationStrategy.getBaseHref();
        ((this._basePath = aT($y(Hy(r)))),
          this._locationStrategy.onPopState((o) => {
            this._subject.next({
              url: this.path(!0),
              pop: !0,
              state: o.state,
              type: o.type,
            });
          }));
      }
      ngOnDestroy() {
        (this._urlChangeSubscription?.unsubscribe(),
          (this._urlChangeListeners = []));
      }
      path(n = !1) {
        return this.normalize(this._locationStrategy.path(n));
      }
      getState() {
        return this._locationStrategy.getState();
      }
      isCurrentPathEqualTo(n, r = "") {
        return this.path() == this.normalize(n + Tt(r));
      }
      normalize(n) {
        return e.stripTrailingSlash(sT(this._basePath, Hy(n)));
      }
      prepareExternalUrl(n) {
        return (
          n && n[0] !== "/" && (n = "/" + n),
          this._locationStrategy.prepareExternalUrl(n)
        );
      }
      go(n, r = "", o = null) {
        (this._locationStrategy.pushState(o, "", n, r),
          this._notifyUrlChangeListeners(
            this.prepareExternalUrl(n + Tt(r)),
            o,
          ));
      }
      replaceState(n, r = "", o = null) {
        (this._locationStrategy.replaceState(o, "", n, r),
          this._notifyUrlChangeListeners(
            this.prepareExternalUrl(n + Tt(r)),
            o,
          ));
      }
      forward() {
        this._locationStrategy.forward();
      }
      back() {
        this._locationStrategy.back();
      }
      historyGo(n = 0) {
        this._locationStrategy.historyGo?.(n);
      }
      onUrlChange(n) {
        return (
          this._urlChangeListeners.push(n),
          (this._urlChangeSubscription ??= this.subscribe((r) => {
            this._notifyUrlChangeListeners(r.url, r.state);
          })),
          () => {
            let r = this._urlChangeListeners.indexOf(n);
            (this._urlChangeListeners.splice(r, 1),
              this._urlChangeListeners.length === 0 &&
                (this._urlChangeSubscription?.unsubscribe(),
                (this._urlChangeSubscription = null)));
          }
        );
      }
      _notifyUrlChangeListeners(n = "", r) {
        this._urlChangeListeners.forEach((o) => o(n, r));
      }
      subscribe(n, r, o) {
        return this._subject.subscribe({
          next: n,
          error: r ?? void 0,
          complete: o ?? void 0,
        });
      }
      static normalizeQueryParams = Tt;
      static joinWithSlash = vd;
      static stripTrailingSlash = $y;
      static ɵfac = function (r) {
        return new (r || e)(b(xt));
      };
      static ɵprov = E({ token: e, factory: () => iT(), providedIn: "root" });
    }
    return e;
  })();
function iT() {
  return new Tr(b(xt));
}
function sT(e, t) {
  if (!e || !t.startsWith(e)) return t;
  let n = t.substring(e.length);
  return n === "" || ["/", ";", "?", "#"].includes(n[0]) ? n : t;
}
function Hy(e) {
  return e.replace(/\/index.html$/, "");
}
function aT(e) {
  if (new RegExp("^(https?:)?//").test(e)) {
    let [, n] = e.split(/\/\/[^\/]+/);
    return n;
  }
  return e;
}
var tv = {
    ADP: [void 0, void 0, 0],
    AFN: [void 0, "\u060B", 0],
    ALL: [void 0, void 0, 0],
    AMD: [void 0, "\u058F", 2],
    AOA: [void 0, "Kz"],
    ARS: [void 0, "$"],
    AUD: ["A$", "$"],
    AZN: [void 0, "\u20BC"],
    BAM: [void 0, "KM"],
    BBD: [void 0, "$"],
    BDT: [void 0, "\u09F3"],
    BHD: [void 0, void 0, 3],
    BIF: [void 0, void 0, 0],
    BMD: [void 0, "$"],
    BND: [void 0, "$"],
    BOB: [void 0, "Bs"],
    BRL: ["R$"],
    BSD: [void 0, "$"],
    BWP: [void 0, "P"],
    BYN: [void 0, void 0, 2],
    BYR: [void 0, void 0, 0],
    BZD: [void 0, "$"],
    CAD: ["CA$", "$", 2],
    CHF: [void 0, void 0, 2],
    CLF: [void 0, void 0, 4],
    CLP: [void 0, "$", 0],
    CNY: ["CN\xA5", "\xA5"],
    COP: [void 0, "$", 2],
    CRC: [void 0, "\u20A1", 2],
    CUC: [void 0, "$"],
    CUP: [void 0, "$"],
    CZK: [void 0, "K\u010D", 2],
    DJF: [void 0, void 0, 0],
    DKK: [void 0, "kr", 2],
    DOP: [void 0, "$"],
    EGP: [void 0, "E\xA3"],
    ESP: [void 0, "\u20A7", 0],
    EUR: ["\u20AC"],
    FJD: [void 0, "$"],
    FKP: [void 0, "\xA3"],
    GBP: ["\xA3"],
    GEL: [void 0, "\u20BE"],
    GHS: [void 0, "GH\u20B5"],
    GIP: [void 0, "\xA3"],
    GNF: [void 0, "FG", 0],
    GTQ: [void 0, "Q"],
    GYD: [void 0, "$", 2],
    HKD: ["HK$", "$"],
    HNL: [void 0, "L"],
    HRK: [void 0, "kn"],
    HUF: [void 0, "Ft", 2],
    IDR: [void 0, "Rp", 2],
    ILS: ["\u20AA"],
    INR: ["\u20B9"],
    IQD: [void 0, void 0, 0],
    IRR: [void 0, void 0, 0],
    ISK: [void 0, "kr", 0],
    ITL: [void 0, void 0, 0],
    JMD: [void 0, "$"],
    JOD: [void 0, void 0, 3],
    JPY: ["\xA5", void 0, 0],
    KHR: [void 0, "\u17DB"],
    KMF: [void 0, "CF", 0],
    KPW: [void 0, "\u20A9", 0],
    KRW: ["\u20A9", void 0, 0],
    KWD: [void 0, void 0, 3],
    KYD: [void 0, "$"],
    KZT: [void 0, "\u20B8"],
    LAK: [void 0, "\u20AD", 0],
    LBP: [void 0, "L\xA3", 0],
    LKR: [void 0, "Rs"],
    LRD: [void 0, "$"],
    LTL: [void 0, "Lt"],
    LUF: [void 0, void 0, 0],
    LVL: [void 0, "Ls"],
    LYD: [void 0, void 0, 3],
    MGA: [void 0, "Ar", 0],
    MGF: [void 0, void 0, 0],
    MMK: [void 0, "K", 0],
    MNT: [void 0, "\u20AE", 2],
    MRO: [void 0, void 0, 0],
    MUR: [void 0, "Rs", 2],
    MXN: ["MX$", "$"],
    MYR: [void 0, "RM"],
    NAD: [void 0, "$"],
    NGN: [void 0, "\u20A6"],
    NIO: [void 0, "C$"],
    NOK: [void 0, "kr", 2],
    NPR: [void 0, "Rs"],
    NZD: ["NZ$", "$"],
    OMR: [void 0, void 0, 3],
    PHP: ["\u20B1"],
    PKR: [void 0, "Rs", 2],
    PLN: [void 0, "z\u0142"],
    PYG: [void 0, "\u20B2", 0],
    RON: [void 0, "lei"],
    RSD: [void 0, void 0, 0],
    RUB: [void 0, "\u20BD"],
    RWF: [void 0, "RF", 0],
    SBD: [void 0, "$"],
    SEK: [void 0, "kr", 2],
    SGD: [void 0, "$"],
    SHP: [void 0, "\xA3"],
    SLE: [void 0, void 0, 2],
    SLL: [void 0, void 0, 0],
    SOS: [void 0, void 0, 0],
    SRD: [void 0, "$"],
    SSP: [void 0, "\xA3"],
    STD: [void 0, void 0, 0],
    STN: [void 0, "Db"],
    SYP: [void 0, "\xA3", 0],
    THB: [void 0, "\u0E3F"],
    TMM: [void 0, void 0, 0],
    TND: [void 0, void 0, 3],
    TOP: [void 0, "T$"],
    TRL: [void 0, void 0, 0],
    TRY: [void 0, "\u20BA"],
    TTD: [void 0, "$"],
    TWD: ["NT$", "$", 2],
    TZS: [void 0, void 0, 2],
    UAH: [void 0, "\u20B4"],
    UGX: [void 0, void 0, 0],
    USD: ["$"],
    UYI: [void 0, void 0, 0],
    UYU: [void 0, "$"],
    UYW: [void 0, void 0, 4],
    UZS: [void 0, void 0, 2],
    VEF: [void 0, "Bs", 2],
    VND: ["\u20AB", void 0, 0],
    VUV: [void 0, void 0, 0],
    XAF: ["FCFA", void 0, 0],
    XCD: ["EC$", "$"],
    XOF: ["F\u202FCFA", void 0, 0],
    XPF: ["CFPF", void 0, 0],
    XXX: ["\xA4"],
    YER: [void 0, void 0, 0],
    ZAR: [void 0, "R"],
    ZMK: [void 0, void 0, 0],
    ZMW: [void 0, "ZK"],
    ZWD: [void 0, void 0, 0],
  },
  nv = (function (e) {
    return (
      (e[(e.Decimal = 0)] = "Decimal"),
      (e[(e.Percent = 1)] = "Percent"),
      (e[(e.Currency = 2)] = "Currency"),
      (e[(e.Scientific = 3)] = "Scientific"),
      e
    );
  })(nv || {});
var _t = {
  Decimal: 0,
  Group: 1,
  List: 2,
  PercentSign: 3,
  PlusSign: 4,
  MinusSign: 5,
  Exponential: 6,
  SuperscriptingExponent: 7,
  PerMille: 8,
  Infinity: 9,
  NaN: 10,
  TimeSeparator: 11,
  CurrencyDecimal: 12,
  CurrencyGroup: 13,
};
function Lo(e, t) {
  let n = Oo(e),
    r = n[qt.NumberSymbols][t];
  if (typeof r > "u") {
    if (t === _t.CurrencyDecimal) return n[qt.NumberSymbols][_t.Decimal];
    if (t === _t.CurrencyGroup) return n[qt.NumberSymbols][_t.Group];
  }
  return r;
}
function cT(e, t) {
  return Oo(e)[qt.NumberFormats][t];
}
function uT(e) {
  return Oo(e)[qt.Currencies];
}
function lT(e, t, n = "en") {
  let r = uT(n)[e] || tv[e] || [],
    o = r[1];
  return t === "narrow" && typeof o == "string" ? o : r[0] || e;
}
var dT = 2;
function fT(e) {
  let t,
    n = tv[e];
  return (n && (t = n[2]), typeof t == "number" ? t : dT);
}
var hT = /^(\d+)?\.((\d+)(-(\d+))?)?$/,
  zy = 22,
  da = ".",
  jo = "0",
  pT = ";",
  gT = ",",
  ud = "#",
  qy = "\xA4";
function mT(e, t, n, r, o, i, s = !1) {
  let a = "",
    c = !1;
  if (!isFinite(e)) a = Lo(n, _t.Infinity);
  else {
    let u = wT(e);
    s && (u = DT(u));
    let l = t.minInt,
      d = t.minFrac,
      h = t.maxFrac;
    if (i) {
      let L = i.match(hT);
      if (L === null) throw new Error(`${i} is not a valid digit info`);
      let H = L[1],
        Ae = L[3],
        st = L[5];
      (H != null && (l = ld(H)),
        Ae != null && (d = ld(Ae)),
        st != null ? (h = ld(st)) : Ae != null && d > h && (h = d));
    }
    ET(u, d, h);
    let f = u.digits,
      p = u.integerLen,
      g = u.exponent,
      y = [];
    for (c = f.every((L) => !L); p < l; p++) f.unshift(0);
    for (; p < 0; p++) f.unshift(0);
    p > 0 ? (y = f.splice(p, f.length)) : ((y = f), (f = [0]));
    let w = [];
    for (
      f.length >= t.lgSize && w.unshift(f.splice(-t.lgSize, f.length).join(""));
      f.length > t.gSize;
    )
      w.unshift(f.splice(-t.gSize, f.length).join(""));
    (f.length && w.unshift(f.join("")),
      (a = w.join(Lo(n, r))),
      y.length && (a += Lo(n, o) + y.join("")),
      g && (a += Lo(n, _t.Exponential) + "+" + g));
  }
  return (
    e < 0 && !c ? (a = t.negPre + a + t.negSuf) : (a = t.posPre + a + t.posSuf),
    a
  );
}
function yT(e, t, n, r, o) {
  let i = cT(t, nv.Currency),
    s = vT(i, Lo(t, _t.MinusSign));
  return (
    (s.minFrac = fT(r)),
    (s.maxFrac = s.minFrac),
    mT(e, s, t, _t.CurrencyGroup, _t.CurrencyDecimal, o)
      .replace(qy, n)
      .replace(qy, "")
      .trim()
  );
}
function vT(e, t = "-") {
  let n = {
      minInt: 1,
      minFrac: 0,
      maxFrac: 0,
      posPre: "",
      posSuf: "",
      negPre: "",
      negSuf: "",
      gSize: 0,
      lgSize: 0,
    },
    r = e.split(pT),
    o = r[0],
    i = r[1],
    s =
      o.indexOf(da) !== -1
        ? o.split(da)
        : [
            o.substring(0, o.lastIndexOf(jo) + 1),
            o.substring(o.lastIndexOf(jo) + 1),
          ],
    a = s[0],
    c = s[1] || "";
  n.posPre = a.substring(0, a.indexOf(ud));
  for (let l = 0; l < c.length; l++) {
    let d = c.charAt(l);
    d === jo
      ? (n.minFrac = n.maxFrac = l + 1)
      : d === ud
        ? (n.maxFrac = l + 1)
        : (n.posSuf += d);
  }
  let u = a.split(gT);
  if (
    ((n.gSize = u[1] ? u[1].length : 0),
    (n.lgSize = u[2] || u[1] ? (u[2] || u[1]).length : 0),
    i)
  ) {
    let l = o.length - n.posPre.length - n.posSuf.length,
      d = i.indexOf(ud);
    ((n.negPre = i.substring(0, d).replace(/'/g, "")),
      (n.negSuf = i.slice(d + l).replace(/'/g, "")));
  } else ((n.negPre = t + n.posPre), (n.negSuf = n.posSuf));
  return n;
}
function DT(e) {
  if (e.digits[0] === 0) return e;
  let t = e.digits.length - e.integerLen;
  return (
    e.exponent
      ? (e.exponent += 2)
      : (t === 0 ? e.digits.push(0, 0) : t === 1 && e.digits.push(0),
        (e.integerLen += 2)),
    e
  );
}
function wT(e) {
  let t = Math.abs(e) + "",
    n = 0,
    r,
    o,
    i,
    s,
    a;
  for (
    (o = t.indexOf(da)) > -1 && (t = t.replace(da, "")),
      (i = t.search(/e/i)) > 0
        ? (o < 0 && (o = i), (o += +t.slice(i + 1)), (t = t.substring(0, i)))
        : o < 0 && (o = t.length),
      i = 0;
    t.charAt(i) === jo;
    i++
  );
  if (i === (a = t.length)) ((r = [0]), (o = 1));
  else {
    for (a--; t.charAt(a) === jo; ) a--;
    for (o -= i, r = [], s = 0; i <= a; i++, s++) r[s] = Number(t.charAt(i));
  }
  return (
    o > zy && ((r = r.splice(0, zy - 1)), (n = o - 1), (o = 1)),
    { digits: r, exponent: n, integerLen: o }
  );
}
function ET(e, t, n) {
  if (t > n)
    throw new Error(
      `The minimum number of digits after fraction (${t}) is higher than the maximum (${n}).`,
    );
  let r = e.digits,
    o = r.length - e.integerLen,
    i = Math.min(Math.max(t, o), n),
    s = i + e.integerLen,
    a = r[s];
  if (s > 0) {
    r.splice(Math.max(e.integerLen, s));
    for (let d = s; d < r.length; d++) r[d] = 0;
  } else {
    ((o = Math.max(0, o)),
      (e.integerLen = 1),
      (r.length = Math.max(1, (s = i + 1))),
      (r[0] = 0));
    for (let d = 1; d < s; d++) r[d] = 0;
  }
  if (a >= 5)
    if (s - 1 < 0) {
      for (let d = 0; d > s; d--) (r.unshift(0), e.integerLen++);
      (r.unshift(1), e.integerLen++);
    } else r[s - 1]++;
  for (; o < Math.max(0, i); o++) r.push(0);
  let c = i !== 0,
    u = t + e.integerLen,
    l = r.reduceRight(function (d, h, f, p) {
      return (
        (h = h + d),
        (p[f] = h < 10 ? h : h - 10),
        c && (p[f] === 0 && f >= u ? p.pop() : (c = !1)),
        h >= 10 ? 1 : 0
      );
    }, 0);
  l && (r.unshift(l), e.integerLen++);
}
function ld(e) {
  let t = parseInt(e);
  if (isNaN(t)) throw new Error("Invalid integer literal when parsing " + e);
  return t;
}
function fa(e, t) {
  t = encodeURIComponent(t);
  for (let n of e.split(";")) {
    let r = n.indexOf("="),
      [o, i] = r == -1 ? [n, ""] : [n.slice(0, r), n.slice(r + 1)];
    if (o.trim() === t) return decodeURIComponent(i);
  }
  return null;
}
var dd = /\s+/,
  Gy = [],
  rv = (() => {
    class e {
      _ngEl;
      _renderer;
      initialClasses = Gy;
      rawClass;
      stateMap = new Map();
      constructor(n, r) {
        ((this._ngEl = n), (this._renderer = r));
      }
      set klass(n) {
        this.initialClasses = n != null ? n.trim().split(dd) : Gy;
      }
      set ngClass(n) {
        this.rawClass = typeof n == "string" ? n.trim().split(dd) : n;
      }
      ngDoCheck() {
        for (let r of this.initialClasses) this._updateState(r, !0);
        let n = this.rawClass;
        if (Array.isArray(n) || n instanceof Set)
          for (let r of n) this._updateState(r, !0);
        else if (n != null)
          for (let r of Object.keys(n)) this._updateState(r, !!n[r]);
        this._applyStateDiff();
      }
      _updateState(n, r) {
        let o = this.stateMap.get(n);
        o !== void 0
          ? (o.enabled !== r && ((o.changed = !0), (o.enabled = r)),
            (o.touched = !0))
          : this.stateMap.set(n, { enabled: r, changed: !0, touched: !0 });
      }
      _applyStateDiff() {
        for (let n of this.stateMap) {
          let r = n[0],
            o = n[1];
          (o.changed
            ? (this._toggleClass(r, o.enabled), (o.changed = !1))
            : o.touched ||
              (o.enabled && this._toggleClass(r, !1), this.stateMap.delete(r)),
            (o.touched = !1));
        }
      }
      _toggleClass(n, r) {
        ((n = n.trim()),
          n.length > 0 &&
            n.split(dd).forEach((o) => {
              r
                ? this._renderer.addClass(this._ngEl.nativeElement, o)
                : this._renderer.removeClass(this._ngEl.nativeElement, o);
            }));
      }
      static ɵfac = function (r) {
        return new (r || e)(q(tt), q(Er));
      };
      static ɵdir = rt({
        type: e,
        selectors: [["", "ngClass", ""]],
        inputs: { klass: [0, "class", "klass"], ngClass: "ngClass" },
      });
    }
    return e;
  })(),
  TL = (() => {
    class e {
      _viewContainerRef;
      ngComponentOutlet = null;
      ngComponentOutletInputs;
      ngComponentOutletInjector;
      ngComponentOutletContent;
      ngComponentOutletNgModule;
      ngComponentOutletNgModuleFactory;
      _componentRef;
      _moduleRef;
      _inputsUsed = new Map();
      get componentInstance() {
        return this._componentRef?.instance ?? null;
      }
      constructor(n) {
        this._viewContainerRef = n;
      }
      _needToReCreateNgModuleInstance(n) {
        return (
          n.ngComponentOutletNgModule !== void 0 ||
          n.ngComponentOutletNgModuleFactory !== void 0
        );
      }
      _needToReCreateComponentInstance(n) {
        return (
          n.ngComponentOutlet !== void 0 ||
          n.ngComponentOutletContent !== void 0 ||
          n.ngComponentOutletInjector !== void 0 ||
          this._needToReCreateNgModuleInstance(n)
        );
      }
      ngOnChanges(n) {
        if (
          this._needToReCreateComponentInstance(n) &&
          (this._viewContainerRef.clear(),
          this._inputsUsed.clear(),
          (this._componentRef = void 0),
          this.ngComponentOutlet)
        ) {
          let r =
            this.ngComponentOutletInjector ||
            this._viewContainerRef.parentInjector;
          (this._needToReCreateNgModuleInstance(n) &&
            (this._moduleRef?.destroy(),
            this.ngComponentOutletNgModule
              ? (this._moduleRef = $m(this.ngComponentOutletNgModule, Wy(r)))
              : this.ngComponentOutletNgModuleFactory
                ? (this._moduleRef =
                    this.ngComponentOutletNgModuleFactory.create(Wy(r)))
                : (this._moduleRef = void 0)),
            (this._componentRef = this._viewContainerRef.createComponent(
              this.ngComponentOutlet,
              {
                injector: r,
                ngModuleRef: this._moduleRef,
                projectableNodes: this.ngComponentOutletContent,
              },
            )));
        }
      }
      ngDoCheck() {
        if (this._componentRef) {
          if (this.ngComponentOutletInputs)
            for (let n of Object.keys(this.ngComponentOutletInputs))
              this._inputsUsed.set(n, !0);
          this._applyInputStateDiff(this._componentRef);
        }
      }
      ngOnDestroy() {
        this._moduleRef?.destroy();
      }
      _applyInputStateDiff(n) {
        for (let [r, o] of this._inputsUsed)
          o
            ? (n.setInput(r, this.ngComponentOutletInputs[r]),
              this._inputsUsed.set(r, !1))
            : (n.setInput(r, void 0), this._inputsUsed.delete(r));
      }
      static ɵfac = function (r) {
        return new (r || e)(q(nt));
      };
      static ɵdir = rt({
        type: e,
        selectors: [["", "ngComponentOutlet", ""]],
        inputs: {
          ngComponentOutlet: "ngComponentOutlet",
          ngComponentOutletInputs: "ngComponentOutletInputs",
          ngComponentOutletInjector: "ngComponentOutletInjector",
          ngComponentOutletContent: "ngComponentOutletContent",
          ngComponentOutletNgModule: "ngComponentOutletNgModule",
          ngComponentOutletNgModuleFactory: "ngComponentOutletNgModuleFactory",
        },
        exportAs: ["ngComponentOutlet"],
        features: [vr],
      });
    }
    return e;
  })();
function Wy(e) {
  return e.get(En).injector;
}
var fd = class {
    $implicit;
    ngForOf;
    index;
    count;
    constructor(t, n, r, o) {
      ((this.$implicit = t),
        (this.ngForOf = n),
        (this.index = r),
        (this.count = o));
    }
    get first() {
      return this.index === 0;
    }
    get last() {
      return this.index === this.count - 1;
    }
    get even() {
      return this.index % 2 === 0;
    }
    get odd() {
      return !this.even;
    }
  },
  _L = (() => {
    class e {
      _viewContainer;
      _template;
      _differs;
      set ngForOf(n) {
        ((this._ngForOf = n), (this._ngForOfDirty = !0));
      }
      set ngForTrackBy(n) {
        this._trackByFn = n;
      }
      get ngForTrackBy() {
        return this._trackByFn;
      }
      _ngForOf = null;
      _ngForOfDirty = !0;
      _differ = null;
      _trackByFn;
      constructor(n, r, o) {
        ((this._viewContainer = n), (this._template = r), (this._differs = o));
      }
      set ngForTemplate(n) {
        n && (this._template = n);
      }
      ngDoCheck() {
        if (this._ngForOfDirty) {
          this._ngForOfDirty = !1;
          let n = this._ngForOf;
          !this._differ &&
            n &&
            (this._differ = this._differs.find(n).create(this.ngForTrackBy));
        }
        if (this._differ) {
          let n = this._differ.diff(this._ngForOf);
          n && this._applyChanges(n);
        }
      }
      _applyChanges(n) {
        let r = this._viewContainer;
        n.forEachOperation((o, i, s) => {
          if (o.previousIndex == null)
            r.createEmbeddedView(
              this._template,
              new fd(o.item, this._ngForOf, -1, -1),
              s === null ? void 0 : s,
            );
          else if (s == null) r.remove(i === null ? void 0 : i);
          else if (i !== null) {
            let a = r.get(i);
            (r.move(a, s), Zy(a, o));
          }
        });
        for (let o = 0, i = r.length; o < i; o++) {
          let a = r.get(o).context;
          ((a.index = o), (a.count = i), (a.ngForOf = this._ngForOf));
        }
        n.forEachIdentityChange((o) => {
          let i = r.get(o.currentIndex);
          Zy(i, o);
        });
      }
      static ngTemplateContextGuard(n, r) {
        return !0;
      }
      static ɵfac = function (r) {
        return new (r || e)(q(nt), q(Dn), q(ad));
      };
      static ɵdir = rt({
        type: e,
        selectors: [["", "ngFor", "", "ngForOf", ""]],
        inputs: {
          ngForOf: "ngForOf",
          ngForTrackBy: "ngForTrackBy",
          ngForTemplate: "ngForTemplate",
        },
      });
    }
    return e;
  })();
function Zy(e, t) {
  e.context.$implicit = t.item;
}
var xL = (() => {
    class e {
      _viewContainer;
      _context = new hd();
      _thenTemplateRef = null;
      _elseTemplateRef = null;
      _thenViewRef = null;
      _elseViewRef = null;
      constructor(n, r) {
        ((this._viewContainer = n), (this._thenTemplateRef = r));
      }
      set ngIf(n) {
        ((this._context.$implicit = this._context.ngIf = n),
          this._updateView());
      }
      set ngIfThen(n) {
        (Yy(n, !1),
          (this._thenTemplateRef = n),
          (this._thenViewRef = null),
          this._updateView());
      }
      set ngIfElse(n) {
        (Yy(n, !1),
          (this._elseTemplateRef = n),
          (this._elseViewRef = null),
          this._updateView());
      }
      _updateView() {
        this._context.$implicit
          ? this._thenViewRef ||
            (this._viewContainer.clear(),
            (this._elseViewRef = null),
            this._thenTemplateRef &&
              (this._thenViewRef = this._viewContainer.createEmbeddedView(
                this._thenTemplateRef,
                this._context,
              )))
          : this._elseViewRef ||
            (this._viewContainer.clear(),
            (this._thenViewRef = null),
            this._elseTemplateRef &&
              (this._elseViewRef = this._viewContainer.createEmbeddedView(
                this._elseTemplateRef,
                this._context,
              )));
      }
      static ngIfUseIfTypeGuard;
      static ngTemplateGuard_ngIf;
      static ngTemplateContextGuard(n, r) {
        return !0;
      }
      static ɵfac = function (r) {
        return new (r || e)(q(nt), q(Dn));
      };
      static ɵdir = rt({
        type: e,
        selectors: [["", "ngIf", ""]],
        inputs: { ngIf: "ngIf", ngIfThen: "ngIfThen", ngIfElse: "ngIfElse" },
      });
    }
    return e;
  })(),
  hd = class {
    $implicit = null;
    ngIf = null;
  };
function Yy(e, t) {
  if (e && !e.createEmbeddedView) throw new v(2020, !1);
}
var NL = (() => {
    class e {
      _ngEl;
      _differs;
      _renderer;
      _ngStyle = null;
      _differ = null;
      constructor(n, r, o) {
        ((this._ngEl = n), (this._differs = r), (this._renderer = o));
      }
      set ngStyle(n) {
        ((this._ngStyle = n),
          !this._differ &&
            n &&
            (this._differ = this._differs.find(n).create()));
      }
      ngDoCheck() {
        if (this._differ) {
          let n = this._differ.diff(this._ngStyle);
          n && this._applyChanges(n);
        }
      }
      _setStyle(n, r) {
        let [o, i] = n.split("."),
          s = o.indexOf("-") === -1 ? void 0 : Je.DashCase;
        r != null
          ? this._renderer.setStyle(
              this._ngEl.nativeElement,
              o,
              i ? `${r}${i}` : r,
              s,
            )
          : this._renderer.removeStyle(this._ngEl.nativeElement, o, s);
      }
      _applyChanges(n) {
        (n.forEachRemovedItem((r) => this._setStyle(r.key, null)),
          n.forEachAddedItem((r) => this._setStyle(r.key, r.currentValue)),
          n.forEachChangedItem((r) => this._setStyle(r.key, r.currentValue)));
      }
      static ɵfac = function (r) {
        return new (r || e)(q(tt), q(cd), q(Er));
      };
      static ɵdir = rt({
        type: e,
        selectors: [["", "ngStyle", ""]],
        inputs: { ngStyle: "ngStyle" },
      });
    }
    return e;
  })(),
  RL = (() => {
    class e {
      _viewContainerRef;
      _viewRef = null;
      ngTemplateOutletContext = null;
      ngTemplateOutlet = null;
      ngTemplateOutletInjector = null;
      constructor(n) {
        this._viewContainerRef = n;
      }
      ngOnChanges(n) {
        if (this._shouldRecreateView(n)) {
          let r = this._viewContainerRef;
          if (
            (this._viewRef && r.remove(r.indexOf(this._viewRef)),
            !this.ngTemplateOutlet)
          ) {
            this._viewRef = null;
            return;
          }
          let o = this._createContextForwardProxy();
          this._viewRef = r.createEmbeddedView(this.ngTemplateOutlet, o, {
            injector: this.ngTemplateOutletInjector ?? void 0,
          });
        }
      }
      _shouldRecreateView(n) {
        return !!n.ngTemplateOutlet || !!n.ngTemplateOutletInjector;
      }
      _createContextForwardProxy() {
        return new Proxy(
          {},
          {
            set: (n, r, o) =>
              this.ngTemplateOutletContext
                ? Reflect.set(this.ngTemplateOutletContext, r, o)
                : !1,
            get: (n, r, o) => {
              if (this.ngTemplateOutletContext)
                return Reflect.get(this.ngTemplateOutletContext, r, o);
            },
          },
        );
      }
      static ɵfac = function (r) {
        return new (r || e)(q(nt));
      };
      static ɵdir = rt({
        type: e,
        selectors: [["", "ngTemplateOutlet", ""]],
        inputs: {
          ngTemplateOutletContext: "ngTemplateOutletContext",
          ngTemplateOutlet: "ngTemplateOutlet",
          ngTemplateOutletInjector: "ngTemplateOutletInjector",
        },
        features: [vr],
      });
    }
    return e;
  })();
function wd(e, t) {
  return new v(2100, !1);
}
var pd = class {
    createSubscription(t, n) {
      return Te(() =>
        t.subscribe({
          next: n,
          error: (r) => {
            throw r;
          },
        }),
      );
    }
    dispose(t) {
      Te(() => t.unsubscribe());
    }
  },
  gd = class {
    createSubscription(t, n) {
      return t.then(n, (r) => {
        throw r;
      });
    }
    dispose(t) {}
  },
  IT = new gd(),
  CT = new pd(),
  AL = (() => {
    class e {
      _ref;
      _latestValue = null;
      markForCheckOnValueUpdate = !0;
      _subscription = null;
      _obj = null;
      _strategy = null;
      constructor(n) {
        this._ref = n;
      }
      ngOnDestroy() {
        (this._subscription && this._dispose(), (this._ref = null));
      }
      transform(n) {
        if (!this._obj) {
          if (n)
            try {
              ((this.markForCheckOnValueUpdate = !1), this._subscribe(n));
            } finally {
              this.markForCheckOnValueUpdate = !0;
            }
          return this._latestValue;
        }
        return n !== this._obj
          ? (this._dispose(), this.transform(n))
          : this._latestValue;
      }
      _subscribe(n) {
        ((this._obj = n),
          (this._strategy = this._selectStrategy(n)),
          (this._subscription = this._strategy.createSubscription(n, (r) =>
            this._updateLatestValue(n, r),
          )));
      }
      _selectStrategy(n) {
        if (Cr(n)) return IT;
        if (Xl(n)) return CT;
        throw wd(e, n);
      }
      _dispose() {
        (this._strategy.dispose(this._subscription),
          (this._latestValue = null),
          (this._subscription = null),
          (this._obj = null));
      }
      _updateLatestValue(n, r) {
        n === this._obj &&
          ((this._latestValue = r),
          this.markForCheckOnValueUpdate && this._ref?.markForCheck());
      }
      static ɵfac = function (r) {
        return new (r || e)(q(br, 16));
      };
      static ɵpipe = ra({ name: "async", type: e, pure: !1 });
    }
    return e;
  })();
var OL = (() => {
  class e {
    transform(n) {
      if (n == null) return null;
      if (typeof n != "string") throw wd(e, n);
      return n.toUpperCase();
    }
    static ɵfac = function (r) {
      return new (r || e)();
    };
    static ɵpipe = ra({ name: "uppercase", type: e, pure: !0 });
  }
  return e;
})();
var kL = (() => {
  class e {
    _locale;
    _defaultCurrencyCode;
    constructor(n, r = "USD") {
      ((this._locale = n), (this._defaultCurrencyCode = r));
    }
    transform(n, r = this._defaultCurrencyCode, o = "symbol", i, s) {
      if (!bT(n)) return null;
      ((s ||= this._locale),
        typeof o == "boolean" && (o = o ? "symbol" : "code"));
      let a = r || this._defaultCurrencyCode;
      o !== "code" &&
        (o === "symbol" || o === "symbol-narrow"
          ? (a = lT(a, o === "symbol" ? "wide" : "narrow", s))
          : (a = o));
      try {
        let c = MT(n);
        return yT(c, s, a, r, i);
      } catch (c) {
        throw wd(e, c.message);
      }
    }
    static ɵfac = function (r) {
      return new (r || e)(q(ua, 16), q(Fy, 16));
    };
    static ɵpipe = ra({ name: "currency", type: e, pure: !0 });
  }
  return e;
})();
function bT(e) {
  return !(e == null || e === "" || e !== e);
}
function MT(e) {
  if (typeof e == "string" && !isNaN(Number(e) - parseFloat(e)))
    return Number(e);
  if (typeof e != "number") throw new Error(`${e} is not a number`);
  return e;
}
var Ed = (() => {
    class e {
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵmod = Ir({ type: e });
      static ɵinj = hr({});
    }
    return e;
  })(),
  Id = "browser",
  ST = "server";
function PL(e) {
  return e === Id;
}
function Cd(e) {
  return e === ST;
}
var bd = (() => {
    class e {
      static ɵprov = E({
        token: e,
        providedIn: "root",
        factory: () => new md(m(ue), window),
      });
    }
    return e;
  })(),
  md = class {
    document;
    window;
    offset = () => [0, 0];
    constructor(t, n) {
      ((this.document = t), (this.window = n));
    }
    setOffset(t) {
      Array.isArray(t) ? (this.offset = () => t) : (this.offset = t);
    }
    getScrollPosition() {
      return [this.window.scrollX, this.window.scrollY];
    }
    scrollToPosition(t) {
      this.window.scrollTo(t[0], t[1]);
    }
    scrollToAnchor(t) {
      let n = TT(this.document, t);
      n && (this.scrollToElement(n), n.focus());
    }
    setHistoryScrollRestoration(t) {
      this.window.history.scrollRestoration = t;
    }
    scrollToElement(t) {
      let n = t.getBoundingClientRect(),
        r = n.left + this.window.pageXOffset,
        o = n.top + this.window.pageYOffset,
        i = this.offset();
      this.window.scrollTo(r - i[0], o - i[1]);
    }
  };
function TT(e, t) {
  let n = e.getElementById(t) || e.getElementsByName(t)[0];
  if (n) return n;
  if (
    typeof e.createTreeWalker == "function" &&
    e.body &&
    typeof e.body.attachShadow == "function"
  ) {
    let r = e.createTreeWalker(e.body, NodeFilter.SHOW_ELEMENT),
      o = r.currentNode;
    for (; o; ) {
      let i = o.shadowRoot;
      if (i) {
        let s = i.getElementById(t) || i.querySelector(`[name="${t}"]`);
        if (s) return s;
      }
      o = r.nextNode();
    }
  }
  return null;
}
var Sr = class {};
var Bo = class {},
  pa = class {},
  Tn = class e {
    headers;
    normalizedNames = new Map();
    lazyInit;
    lazyUpdate = null;
    constructor(t) {
      t
        ? typeof t == "string"
          ? (this.lazyInit = () => {
              ((this.headers = new Map()),
                t
                  .split(
                    `
`,
                  )
                  .forEach((n) => {
                    let r = n.indexOf(":");
                    if (r > 0) {
                      let o = n.slice(0, r),
                        i = n.slice(r + 1).trim();
                      this.addHeaderEntry(o, i);
                    }
                  }));
            })
          : typeof Headers < "u" && t instanceof Headers
            ? ((this.headers = new Map()),
              t.forEach((n, r) => {
                this.addHeaderEntry(r, n);
              }))
            : (this.lazyInit = () => {
                ((this.headers = new Map()),
                  Object.entries(t).forEach(([n, r]) => {
                    this.setHeaderEntries(n, r);
                  }));
              })
        : (this.headers = new Map());
    }
    has(t) {
      return (this.init(), this.headers.has(t.toLowerCase()));
    }
    get(t) {
      this.init();
      let n = this.headers.get(t.toLowerCase());
      return n && n.length > 0 ? n[0] : null;
    }
    keys() {
      return (this.init(), Array.from(this.normalizedNames.values()));
    }
    getAll(t) {
      return (this.init(), this.headers.get(t.toLowerCase()) || null);
    }
    append(t, n) {
      return this.clone({ name: t, value: n, op: "a" });
    }
    set(t, n) {
      return this.clone({ name: t, value: n, op: "s" });
    }
    delete(t, n) {
      return this.clone({ name: t, value: n, op: "d" });
    }
    maybeSetNormalizedName(t, n) {
      this.normalizedNames.has(n) || this.normalizedNames.set(n, t);
    }
    init() {
      this.lazyInit &&
        (this.lazyInit instanceof e
          ? this.copyFrom(this.lazyInit)
          : this.lazyInit(),
        (this.lazyInit = null),
        this.lazyUpdate &&
          (this.lazyUpdate.forEach((t) => this.applyUpdate(t)),
          (this.lazyUpdate = null)));
    }
    copyFrom(t) {
      (t.init(),
        Array.from(t.headers.keys()).forEach((n) => {
          (this.headers.set(n, t.headers.get(n)),
            this.normalizedNames.set(n, t.normalizedNames.get(n)));
        }));
    }
    clone(t) {
      let n = new e();
      return (
        (n.lazyInit =
          this.lazyInit && this.lazyInit instanceof e ? this.lazyInit : this),
        (n.lazyUpdate = (this.lazyUpdate || []).concat([t])),
        n
      );
    }
    applyUpdate(t) {
      let n = t.name.toLowerCase();
      switch (t.op) {
        case "a":
        case "s":
          let r = t.value;
          if ((typeof r == "string" && (r = [r]), r.length === 0)) return;
          this.maybeSetNormalizedName(t.name, n);
          let o = (t.op === "a" ? this.headers.get(n) : void 0) || [];
          (o.push(...r), this.headers.set(n, o));
          break;
        case "d":
          let i = t.value;
          if (!i) (this.headers.delete(n), this.normalizedNames.delete(n));
          else {
            let s = this.headers.get(n);
            if (!s) return;
            ((s = s.filter((a) => i.indexOf(a) === -1)),
              s.length === 0
                ? (this.headers.delete(n), this.normalizedNames.delete(n))
                : this.headers.set(n, s));
          }
          break;
      }
    }
    addHeaderEntry(t, n) {
      let r = t.toLowerCase();
      (this.maybeSetNormalizedName(t, r),
        this.headers.has(r)
          ? this.headers.get(r).push(n)
          : this.headers.set(r, [n]));
    }
    setHeaderEntries(t, n) {
      let r = (Array.isArray(n) ? n : [n]).map((i) => i.toString()),
        o = t.toLowerCase();
      (this.headers.set(o, r), this.maybeSetNormalizedName(t, o));
    }
    forEach(t) {
      (this.init(),
        Array.from(this.normalizedNames.keys()).forEach((n) =>
          t(this.normalizedNames.get(n), this.headers.get(n)),
        ));
    }
  };
var Td = class {
  encodeKey(t) {
    return ov(t);
  }
  encodeValue(t) {
    return ov(t);
  }
  decodeKey(t) {
    return decodeURIComponent(t);
  }
  decodeValue(t) {
    return decodeURIComponent(t);
  }
};
function _T(e, t) {
  let n = new Map();
  return (
    e.length > 0 &&
      e
        .replace(/^\?/, "")
        .split("&")
        .forEach((o) => {
          let i = o.indexOf("="),
            [s, a] =
              i == -1
                ? [t.decodeKey(o), ""]
                : [t.decodeKey(o.slice(0, i)), t.decodeValue(o.slice(i + 1))],
            c = n.get(s) || [];
          (c.push(a), n.set(s, c));
        }),
    n
  );
}
var xT = /%(\d[a-f0-9])/gi,
  NT = {
    40: "@",
    "3A": ":",
    24: "$",
    "2C": ",",
    "3B": ";",
    "3D": "=",
    "3F": "?",
    "2F": "/",
  };
function ov(e) {
  return encodeURIComponent(e).replace(xT, (t, n) => NT[n] ?? t);
}
function ha(e) {
  return `${e}`;
}
var Gt = class e {
  map;
  encoder;
  updates = null;
  cloneFrom = null;
  constructor(t = {}) {
    if (((this.encoder = t.encoder || new Td()), t.fromString)) {
      if (t.fromObject) throw new v(2805, !1);
      this.map = _T(t.fromString, this.encoder);
    } else
      t.fromObject
        ? ((this.map = new Map()),
          Object.keys(t.fromObject).forEach((n) => {
            let r = t.fromObject[n],
              o = Array.isArray(r) ? r.map(ha) : [ha(r)];
            this.map.set(n, o);
          }))
        : (this.map = null);
  }
  has(t) {
    return (this.init(), this.map.has(t));
  }
  get(t) {
    this.init();
    let n = this.map.get(t);
    return n ? n[0] : null;
  }
  getAll(t) {
    return (this.init(), this.map.get(t) || null);
  }
  keys() {
    return (this.init(), Array.from(this.map.keys()));
  }
  append(t, n) {
    return this.clone({ param: t, value: n, op: "a" });
  }
  appendAll(t) {
    let n = [];
    return (
      Object.keys(t).forEach((r) => {
        let o = t[r];
        Array.isArray(o)
          ? o.forEach((i) => {
              n.push({ param: r, value: i, op: "a" });
            })
          : n.push({ param: r, value: o, op: "a" });
      }),
      this.clone(n)
    );
  }
  set(t, n) {
    return this.clone({ param: t, value: n, op: "s" });
  }
  delete(t, n) {
    return this.clone({ param: t, value: n, op: "d" });
  }
  toString() {
    return (
      this.init(),
      this.keys()
        .map((t) => {
          let n = this.encoder.encodeKey(t);
          return this.map
            .get(t)
            .map((r) => n + "=" + this.encoder.encodeValue(r))
            .join("&");
        })
        .filter((t) => t !== "")
        .join("&")
    );
  }
  clone(t) {
    let n = new e({ encoder: this.encoder });
    return (
      (n.cloneFrom = this.cloneFrom || this),
      (n.updates = (this.updates || []).concat(t)),
      n
    );
  }
  init() {
    (this.map === null && (this.map = new Map()),
      this.cloneFrom !== null &&
        (this.cloneFrom.init(),
        this.cloneFrom
          .keys()
          .forEach((t) => this.map.set(t, this.cloneFrom.map.get(t))),
        this.updates.forEach((t) => {
          switch (t.op) {
            case "a":
            case "s":
              let n = (t.op === "a" ? this.map.get(t.param) : void 0) || [];
              (n.push(ha(t.value)), this.map.set(t.param, n));
              break;
            case "d":
              if (t.value !== void 0) {
                let r = this.map.get(t.param) || [],
                  o = r.indexOf(ha(t.value));
                (o !== -1 && r.splice(o, 1),
                  r.length > 0
                    ? this.map.set(t.param, r)
                    : this.map.delete(t.param));
              } else {
                this.map.delete(t.param);
                break;
              }
          }
        }),
        (this.cloneFrom = this.updates = null)));
  }
};
var _d = class {
  map = new Map();
  set(t, n) {
    return (this.map.set(t, n), this);
  }
  get(t) {
    return (
      this.map.has(t) || this.map.set(t, t.defaultValue()),
      this.map.get(t)
    );
  }
  delete(t) {
    return (this.map.delete(t), this);
  }
  has(t) {
    return this.map.has(t);
  }
  keys() {
    return this.map.keys();
  }
};
function RT(e) {
  switch (e) {
    case "DELETE":
    case "GET":
    case "HEAD":
    case "OPTIONS":
    case "JSONP":
      return !1;
    default:
      return !0;
  }
}
function iv(e) {
  return typeof ArrayBuffer < "u" && e instanceof ArrayBuffer;
}
function sv(e) {
  return typeof Blob < "u" && e instanceof Blob;
}
function av(e) {
  return typeof FormData < "u" && e instanceof FormData;
}
function AT(e) {
  return typeof URLSearchParams < "u" && e instanceof URLSearchParams;
}
var cv = "Content-Type",
  uv = "Accept",
  fv = "X-Request-URL",
  hv = "text/plain",
  pv = "application/json",
  OT = `${pv}, ${hv}, */*`,
  Vo = class e {
    url;
    body = null;
    headers;
    context;
    reportProgress = !1;
    withCredentials = !1;
    responseType = "json";
    method;
    params;
    urlWithParams;
    transferCache;
    constructor(t, n, r, o) {
      ((this.url = n), (this.method = t.toUpperCase()));
      let i;
      if (
        (RT(this.method) || o
          ? ((this.body = r !== void 0 ? r : null), (i = o))
          : (i = r),
        i &&
          ((this.reportProgress = !!i.reportProgress),
          (this.withCredentials = !!i.withCredentials),
          i.responseType && (this.responseType = i.responseType),
          i.headers && (this.headers = i.headers),
          i.context && (this.context = i.context),
          i.params && (this.params = i.params),
          (this.transferCache = i.transferCache)),
        (this.headers ??= new Tn()),
        (this.context ??= new _d()),
        !this.params)
      )
        ((this.params = new Gt()), (this.urlWithParams = n));
      else {
        let s = this.params.toString();
        if (s.length === 0) this.urlWithParams = n;
        else {
          let a = n.indexOf("?"),
            c = a === -1 ? "?" : a < n.length - 1 ? "&" : "";
          this.urlWithParams = n + c + s;
        }
      }
    }
    serializeBody() {
      return this.body === null
        ? null
        : typeof this.body == "string" ||
            iv(this.body) ||
            sv(this.body) ||
            av(this.body) ||
            AT(this.body)
          ? this.body
          : this.body instanceof Gt
            ? this.body.toString()
            : typeof this.body == "object" ||
                typeof this.body == "boolean" ||
                Array.isArray(this.body)
              ? JSON.stringify(this.body)
              : this.body.toString();
    }
    detectContentTypeHeader() {
      return this.body === null || av(this.body)
        ? null
        : sv(this.body)
          ? this.body.type || null
          : iv(this.body)
            ? null
            : typeof this.body == "string"
              ? hv
              : this.body instanceof Gt
                ? "application/x-www-form-urlencoded;charset=UTF-8"
                : typeof this.body == "object" ||
                    typeof this.body == "number" ||
                    typeof this.body == "boolean"
                  ? pv
                  : null;
    }
    clone(t = {}) {
      let n = t.method || this.method,
        r = t.url || this.url,
        o = t.responseType || this.responseType,
        i = t.transferCache ?? this.transferCache,
        s = t.body !== void 0 ? t.body : this.body,
        a = t.withCredentials ?? this.withCredentials,
        c = t.reportProgress ?? this.reportProgress,
        u = t.headers || this.headers,
        l = t.params || this.params,
        d = t.context ?? this.context;
      return (
        t.setHeaders !== void 0 &&
          (u = Object.keys(t.setHeaders).reduce(
            (h, f) => h.set(f, t.setHeaders[f]),
            u,
          )),
        t.setParams &&
          (l = Object.keys(t.setParams).reduce(
            (h, f) => h.set(f, t.setParams[f]),
            l,
          )),
        new e(n, r, s, {
          params: l,
          headers: u,
          context: d,
          reportProgress: c,
          responseType: o,
          withCredentials: a,
          transferCache: i,
        })
      );
    }
  },
  _r = (function (e) {
    return (
      (e[(e.Sent = 0)] = "Sent"),
      (e[(e.UploadProgress = 1)] = "UploadProgress"),
      (e[(e.ResponseHeader = 2)] = "ResponseHeader"),
      (e[(e.DownloadProgress = 3)] = "DownloadProgress"),
      (e[(e.Response = 4)] = "Response"),
      (e[(e.User = 5)] = "User"),
      e
    );
  })(_r || {}),
  Uo = class {
    headers;
    status;
    statusText;
    url;
    ok;
    type;
    constructor(t, n = 200, r = "OK") {
      ((this.headers = t.headers || new Tn()),
        (this.status = t.status !== void 0 ? t.status : n),
        (this.statusText = t.statusText || r),
        (this.url = t.url || null),
        (this.ok = this.status >= 200 && this.status < 300));
    }
  },
  xd = class e extends Uo {
    constructor(t = {}) {
      super(t);
    }
    type = _r.ResponseHeader;
    clone(t = {}) {
      return new e({
        headers: t.headers || this.headers,
        status: t.status !== void 0 ? t.status : this.status,
        statusText: t.statusText || this.statusText,
        url: t.url || this.url || void 0,
      });
    }
  },
  ga = class e extends Uo {
    body;
    constructor(t = {}) {
      (super(t), (this.body = t.body !== void 0 ? t.body : null));
    }
    type = _r.Response;
    clone(t = {}) {
      return new e({
        body: t.body !== void 0 ? t.body : this.body,
        headers: t.headers || this.headers,
        status: t.status !== void 0 ? t.status : this.status,
        statusText: t.statusText || this.statusText,
        url: t.url || this.url || void 0,
      });
    }
  },
  ma = class extends Uo {
    name = "HttpErrorResponse";
    message;
    error;
    ok = !1;
    constructor(t) {
      (super(t, 0, "Unknown Error"),
        this.status >= 200 && this.status < 300
          ? (this.message = `Http failure during parsing for ${t.url || "(unknown url)"}`)
          : (this.message = `Http failure response for ${t.url || "(unknown url)"}: ${t.status} ${t.statusText}`),
        (this.error = t.error || null));
    }
  },
  kT = 200,
  PT = 204;
function Sd(e, t) {
  return {
    body: t,
    headers: e.headers,
    context: e.context,
    observe: e.observe,
    params: e.params,
    reportProgress: e.reportProgress,
    responseType: e.responseType,
    withCredentials: e.withCredentials,
    transferCache: e.transferCache,
  };
}
var FT = (() => {
  class e {
    handler;
    constructor(n) {
      this.handler = n;
    }
    request(n, r, o = {}) {
      let i;
      if (n instanceof Vo) i = n;
      else {
        let c;
        o.headers instanceof Tn ? (c = o.headers) : (c = new Tn(o.headers));
        let u;
        (o.params &&
          (o.params instanceof Gt
            ? (u = o.params)
            : (u = new Gt({ fromObject: o.params }))),
          (i = new Vo(n, r, o.body !== void 0 ? o.body : null, {
            headers: c,
            context: o.context,
            params: u,
            reportProgress: o.reportProgress,
            responseType: o.responseType || "json",
            withCredentials: o.withCredentials,
            transferCache: o.transferCache,
          })));
      }
      let s = S(i).pipe(lt((c) => this.handler.handle(c)));
      if (n instanceof Vo || o.observe === "events") return s;
      let a = s.pipe(ge((c) => c instanceof ga));
      switch (o.observe || "body") {
        case "body":
          switch (i.responseType) {
            case "arraybuffer":
              return a.pipe(
                O((c) => {
                  if (c.body !== null && !(c.body instanceof ArrayBuffer))
                    throw new v(2806, !1);
                  return c.body;
                }),
              );
            case "blob":
              return a.pipe(
                O((c) => {
                  if (c.body !== null && !(c.body instanceof Blob))
                    throw new v(2807, !1);
                  return c.body;
                }),
              );
            case "text":
              return a.pipe(
                O((c) => {
                  if (c.body !== null && typeof c.body != "string")
                    throw new v(2808, !1);
                  return c.body;
                }),
              );
            case "json":
            default:
              return a.pipe(O((c) => c.body));
          }
        case "response":
          return a;
        default:
          throw new v(2809, !1);
      }
    }
    delete(n, r = {}) {
      return this.request("DELETE", n, r);
    }
    get(n, r = {}) {
      return this.request("GET", n, r);
    }
    head(n, r = {}) {
      return this.request("HEAD", n, r);
    }
    jsonp(n, r) {
      return this.request("JSONP", n, {
        params: new Gt().append(r, "JSONP_CALLBACK"),
        observe: "body",
        responseType: "json",
      });
    }
    options(n, r = {}) {
      return this.request("OPTIONS", n, r);
    }
    patch(n, r, o = {}) {
      return this.request("PATCH", n, Sd(o, r));
    }
    post(n, r, o = {}) {
      return this.request("POST", n, Sd(o, r));
    }
    put(n, r, o = {}) {
      return this.request("PUT", n, Sd(o, r));
    }
    static ɵfac = function (r) {
      return new (r || e)(b(Bo));
    };
    static ɵprov = E({ token: e, factory: e.ɵfac });
  }
  return e;
})();
var LT = new I("");
function jT(e, t) {
  return t(e);
}
function VT(e, t, n) {
  return (r, o) => Me(n, () => t(r, (i) => e(i, o)));
}
var gv = new I(""),
  BT = new I(""),
  UT = new I("", { providedIn: "root", factory: () => !0 });
var lv = (() => {
  class e extends Bo {
    backend;
    injector;
    chain = null;
    pendingTasks = m(Ct);
    contributeToStability = m(UT);
    constructor(n, r) {
      (super(), (this.backend = n), (this.injector = r));
    }
    handle(n) {
      if (this.chain === null) {
        let r = Array.from(
          new Set([...this.injector.get(gv), ...this.injector.get(BT, [])]),
        );
        this.chain = r.reduceRight((o, i) => VT(o, i, this.injector), jT);
      }
      if (this.contributeToStability) {
        let r = this.pendingTasks.add();
        return this.chain(n, (o) => this.backend.handle(o)).pipe(
          an(() => this.pendingTasks.remove(r)),
        );
      } else return this.chain(n, (r) => this.backend.handle(r));
    }
    static ɵfac = function (r) {
      return new (r || e)(b(pa), b(ye));
    };
    static ɵprov = E({ token: e, factory: e.ɵfac });
  }
  return e;
})();
var $T = /^\)\]\}',?\n/,
  HT = RegExp(`^${fv}:`, "m");
function zT(e) {
  return "responseURL" in e && e.responseURL
    ? e.responseURL
    : HT.test(e.getAllResponseHeaders())
      ? e.getResponseHeader(fv)
      : null;
}
var dv = (() => {
    class e {
      xhrFactory;
      constructor(n) {
        this.xhrFactory = n;
      }
      handle(n) {
        if (n.method === "JSONP") throw new v(-2800, !1);
        let r = this.xhrFactory;
        return (r.ɵloadImpl ? G(r.ɵloadImpl()) : S(null)).pipe(
          me(
            () =>
              new F((i) => {
                let s = r.build();
                if (
                  (s.open(n.method, n.urlWithParams),
                  n.withCredentials && (s.withCredentials = !0),
                  n.headers.forEach((g, y) =>
                    s.setRequestHeader(g, y.join(",")),
                  ),
                  n.headers.has(uv) || s.setRequestHeader(uv, OT),
                  !n.headers.has(cv))
                ) {
                  let g = n.detectContentTypeHeader();
                  g !== null && s.setRequestHeader(cv, g);
                }
                if (n.responseType) {
                  let g = n.responseType.toLowerCase();
                  s.responseType = g !== "json" ? g : "text";
                }
                let a = n.serializeBody(),
                  c = null,
                  u = () => {
                    if (c !== null) return c;
                    let g = s.statusText || "OK",
                      y = new Tn(s.getAllResponseHeaders()),
                      w = zT(s) || n.url;
                    return (
                      (c = new xd({
                        headers: y,
                        status: s.status,
                        statusText: g,
                        url: w,
                      })),
                      c
                    );
                  },
                  l = () => {
                    let { headers: g, status: y, statusText: w, url: L } = u(),
                      H = null;
                    (y !== PT &&
                      (H =
                        typeof s.response > "u" ? s.responseText : s.response),
                      y === 0 && (y = H ? kT : 0));
                    let Ae = y >= 200 && y < 300;
                    if (n.responseType === "json" && typeof H == "string") {
                      let st = H;
                      H = H.replace($T, "");
                      try {
                        H = H !== "" ? JSON.parse(H) : null;
                      } catch ($r) {
                        ((H = st),
                          Ae && ((Ae = !1), (H = { error: $r, text: H })));
                      }
                    }
                    Ae
                      ? (i.next(
                          new ga({
                            body: H,
                            headers: g,
                            status: y,
                            statusText: w,
                            url: L || void 0,
                          }),
                        ),
                        i.complete())
                      : i.error(
                          new ma({
                            error: H,
                            headers: g,
                            status: y,
                            statusText: w,
                            url: L || void 0,
                          }),
                        );
                  },
                  d = (g) => {
                    let { url: y } = u(),
                      w = new ma({
                        error: g,
                        status: s.status || 0,
                        statusText: s.statusText || "Unknown Error",
                        url: y || void 0,
                      });
                    i.error(w);
                  },
                  h = !1,
                  f = (g) => {
                    h || (i.next(u()), (h = !0));
                    let y = { type: _r.DownloadProgress, loaded: g.loaded };
                    (g.lengthComputable && (y.total = g.total),
                      n.responseType === "text" &&
                        s.responseText &&
                        (y.partialText = s.responseText),
                      i.next(y));
                  },
                  p = (g) => {
                    let y = { type: _r.UploadProgress, loaded: g.loaded };
                    (g.lengthComputable && (y.total = g.total), i.next(y));
                  };
                return (
                  s.addEventListener("load", l),
                  s.addEventListener("error", d),
                  s.addEventListener("timeout", d),
                  s.addEventListener("abort", d),
                  n.reportProgress &&
                    (s.addEventListener("progress", f),
                    a !== null &&
                      s.upload &&
                      s.upload.addEventListener("progress", p)),
                  s.send(a),
                  i.next({ type: _r.Sent }),
                  () => {
                    (s.removeEventListener("error", d),
                      s.removeEventListener("abort", d),
                      s.removeEventListener("load", l),
                      s.removeEventListener("timeout", d),
                      n.reportProgress &&
                        (s.removeEventListener("progress", f),
                        a !== null &&
                          s.upload &&
                          s.upload.removeEventListener("progress", p)),
                      s.readyState !== s.DONE && s.abort());
                  }
                );
              }),
          ),
        );
      }
      static ɵfac = function (r) {
        return new (r || e)(b(Sr));
      };
      static ɵprov = E({ token: e, factory: e.ɵfac });
    }
    return e;
  })(),
  mv = new I(""),
  qT = "XSRF-TOKEN",
  GT = new I("", { providedIn: "root", factory: () => qT }),
  WT = "X-XSRF-TOKEN",
  ZT = new I("", { providedIn: "root", factory: () => WT }),
  ya = class {},
  YT = (() => {
    class e {
      doc;
      platform;
      cookieName;
      lastCookieString = "";
      lastToken = null;
      parseCount = 0;
      constructor(n, r, o) {
        ((this.doc = n), (this.platform = r), (this.cookieName = o));
      }
      getToken() {
        if (this.platform === "server") return null;
        let n = this.doc.cookie || "";
        return (
          n !== this.lastCookieString &&
            (this.parseCount++,
            (this.lastToken = fa(n, this.cookieName)),
            (this.lastCookieString = n)),
          this.lastToken
        );
      }
      static ɵfac = function (r) {
        return new (r || e)(b(ue), b(bn), b(GT));
      };
      static ɵprov = E({ token: e, factory: e.ɵfac });
    }
    return e;
  })();
function QT(e, t) {
  let n = e.url.toLowerCase();
  if (
    !m(mv) ||
    e.method === "GET" ||
    e.method === "HEAD" ||
    n.startsWith("http://") ||
    n.startsWith("https://")
  )
    return t(e);
  let r = m(ya).getToken(),
    o = m(ZT);
  return (
    r != null &&
      !e.headers.has(o) &&
      (e = e.clone({ headers: e.headers.set(o, r) })),
    t(e)
  );
}
function QL(...e) {
  let t = [
    FT,
    dv,
    lv,
    { provide: Bo, useExisting: lv },
    { provide: pa, useFactory: () => m(LT, { optional: !0 }) ?? m(dv) },
    { provide: gv, useValue: QT, multi: !0 },
    { provide: mv, useValue: !0 },
    { provide: ya, useClass: YT },
  ];
  for (let n of e) t.push(...n.ɵproviders);
  return pr(t);
}
var Rd = class extends la {
    supportsDOMEvents = !0;
  },
  Ad = class e extends Rd {
    static makeCurrent() {
      Ky(new e());
    }
    onAndCancel(t, n, r, o) {
      return (
        t.addEventListener(n, r, o),
        () => {
          t.removeEventListener(n, r, o);
        }
      );
    }
    dispatchEvent(t, n) {
      t.dispatchEvent(n);
    }
    remove(t) {
      t.remove();
    }
    createElement(t, n) {
      return ((n = n || this.getDefaultDocument()), n.createElement(t));
    }
    createHtmlDocument() {
      return document.implementation.createHTMLDocument("fakeTitle");
    }
    getDefaultDocument() {
      return document;
    }
    isElementNode(t) {
      return t.nodeType === Node.ELEMENT_NODE;
    }
    isShadowRoot(t) {
      return t instanceof DocumentFragment;
    }
    getGlobalEventTarget(t, n) {
      return n === "window"
        ? window
        : n === "document"
          ? t
          : n === "body"
            ? t.body
            : null;
    }
    getBaseHref(t) {
      let n = KT();
      return n == null ? null : JT(n);
    }
    resetBaseElement() {
      $o = null;
    }
    getUserAgent() {
      return window.navigator.userAgent;
    }
    getCookie(t) {
      return fa(document.cookie, t);
    }
  },
  $o = null;
function KT() {
  return (
    ($o = $o || document.querySelector("base")),
    $o ? $o.getAttribute("href") : null
  );
}
function JT(e) {
  return new URL(e, document.baseURI).pathname;
}
var XT = (() => {
    class e {
      build() {
        return new XMLHttpRequest();
      }
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵprov = E({ token: e, factory: e.ɵfac });
    }
    return e;
  })(),
  Od = new I(""),
  Iv = (() => {
    class e {
      _zone;
      _plugins;
      _eventNameToPlugin = new Map();
      constructor(n, r) {
        ((this._zone = r),
          n.forEach((o) => {
            o.manager = this;
          }),
          (this._plugins = n.slice().reverse()));
      }
      addEventListener(n, r, o, i) {
        return this._findPluginFor(r).addEventListener(n, r, o, i);
      }
      getZone() {
        return this._zone;
      }
      _findPluginFor(n) {
        let r = this._eventNameToPlugin.get(n);
        if (r) return r;
        if (((r = this._plugins.find((i) => i.supports(n))), !r))
          throw new v(5101, !1);
        return (this._eventNameToPlugin.set(n, r), r);
      }
      static ɵfac = function (r) {
        return new (r || e)(b(Od), b(J));
      };
      static ɵprov = E({ token: e, factory: e.ɵfac });
    }
    return e;
  })(),
  Da = class {
    _doc;
    constructor(t) {
      this._doc = t;
    }
    manager;
  },
  va = "ng-app-id";
function yv(e) {
  for (let t of e) t.remove();
}
function vv(e, t) {
  let n = t.createElement("style");
  return ((n.textContent = e), n);
}
function e_(e, t, n, r) {
  let o = e.head?.querySelectorAll(`style[${va}="${t}"],link[${va}="${t}"]`);
  if (o)
    for (let i of o)
      (i.removeAttribute(va),
        i instanceof HTMLLinkElement
          ? r.set(i.href.slice(i.href.lastIndexOf("/") + 1), {
              usage: 0,
              elements: [i],
            })
          : i.textContent && n.set(i.textContent, { usage: 0, elements: [i] }));
}
function kd(e, t) {
  let n = t.createElement("link");
  return (n.setAttribute("rel", "stylesheet"), n.setAttribute("href", e), n);
}
var Cv = (() => {
    class e {
      doc;
      appId;
      nonce;
      inline = new Map();
      external = new Map();
      hosts = new Set();
      isServer;
      constructor(n, r, o, i = {}) {
        ((this.doc = n),
          (this.appId = r),
          (this.nonce = o),
          (this.isServer = Cd(i)),
          e_(n, r, this.inline, this.external),
          this.hosts.add(n.head));
      }
      addStyles(n, r) {
        for (let o of n) this.addUsage(o, this.inline, vv);
        r?.forEach((o) => this.addUsage(o, this.external, kd));
      }
      removeStyles(n, r) {
        for (let o of n) this.removeUsage(o, this.inline);
        r?.forEach((o) => this.removeUsage(o, this.external));
      }
      addUsage(n, r, o) {
        let i = r.get(n);
        i
          ? i.usage++
          : r.set(n, {
              usage: 1,
              elements: [...this.hosts].map((s) =>
                this.addElement(s, o(n, this.doc)),
              ),
            });
      }
      removeUsage(n, r) {
        let o = r.get(n);
        o && (o.usage--, o.usage <= 0 && (yv(o.elements), r.delete(n)));
      }
      ngOnDestroy() {
        for (let [, { elements: n }] of [...this.inline, ...this.external])
          yv(n);
        this.hosts.clear();
      }
      addHost(n) {
        this.hosts.add(n);
        for (let [r, { elements: o }] of this.inline)
          o.push(this.addElement(n, vv(r, this.doc)));
        for (let [r, { elements: o }] of this.external)
          o.push(this.addElement(n, kd(r, this.doc)));
      }
      removeHost(n) {
        this.hosts.delete(n);
      }
      addElement(n, r) {
        return (
          this.nonce && r.setAttribute("nonce", this.nonce),
          this.isServer && r.setAttribute(va, this.appId),
          n.appendChild(r)
        );
      }
      static ɵfac = function (r) {
        return new (r || e)(b(ue), b(Dl), b(El, 8), b(bn));
      };
      static ɵprov = E({ token: e, factory: e.ɵfac });
    }
    return e;
  })(),
  Nd = {
    svg: "http://www.w3.org/2000/svg",
    xhtml: "http://www.w3.org/1999/xhtml",
    xlink: "http://www.w3.org/1999/xlink",
    xml: "http://www.w3.org/XML/1998/namespace",
    xmlns: "http://www.w3.org/2000/xmlns/",
    math: "http://www.w3.org/1998/Math/MathML",
  },
  Fd = /%COMP%/g;
var bv = "%COMP%",
  t_ = `_nghost-${bv}`,
  n_ = `_ngcontent-${bv}`,
  r_ = !0,
  o_ = new I("", { providedIn: "root", factory: () => r_ });
function i_(e) {
  return n_.replace(Fd, e);
}
function s_(e) {
  return t_.replace(Fd, e);
}
function Mv(e, t) {
  return t.map((n) => n.replace(Fd, e));
}
var Dv = (() => {
    class e {
      eventManager;
      sharedStylesHost;
      appId;
      removeStylesOnCompDestroy;
      doc;
      platformId;
      ngZone;
      nonce;
      tracingService;
      rendererByCompId = new Map();
      defaultRenderer;
      platformIsServer;
      constructor(n, r, o, i, s, a, c, u = null, l = null) {
        ((this.eventManager = n),
          (this.sharedStylesHost = r),
          (this.appId = o),
          (this.removeStylesOnCompDestroy = i),
          (this.doc = s),
          (this.platformId = a),
          (this.ngZone = c),
          (this.nonce = u),
          (this.tracingService = l),
          (this.platformIsServer = Cd(a)),
          (this.defaultRenderer = new Ho(
            n,
            s,
            c,
            this.platformIsServer,
            this.tracingService,
          )));
      }
      createRenderer(n, r) {
        if (!n || !r) return this.defaultRenderer;
        this.platformIsServer &&
          r.encapsulation === Ke.ShadowDom &&
          (r = j(D({}, r), { encapsulation: Ke.Emulated }));
        let o = this.getOrCreateRenderer(n, r);
        return (
          o instanceof wa
            ? o.applyToHost(n)
            : o instanceof zo && o.applyStyles(),
          o
        );
      }
      getOrCreateRenderer(n, r) {
        let o = this.rendererByCompId,
          i = o.get(r.id);
        if (!i) {
          let s = this.doc,
            a = this.ngZone,
            c = this.eventManager,
            u = this.sharedStylesHost,
            l = this.removeStylesOnCompDestroy,
            d = this.platformIsServer,
            h = this.tracingService;
          switch (r.encapsulation) {
            case Ke.Emulated:
              i = new wa(c, u, r, this.appId, l, s, a, d, h);
              break;
            case Ke.ShadowDom:
              return new Pd(c, u, n, r, s, a, this.nonce, d, h);
            default:
              i = new zo(c, u, r, l, s, a, d, h);
              break;
          }
          o.set(r.id, i);
        }
        return i;
      }
      ngOnDestroy() {
        this.rendererByCompId.clear();
      }
      componentReplaced(n) {
        this.rendererByCompId.delete(n);
      }
      static ɵfac = function (r) {
        return new (r || e)(
          b(Iv),
          b(Cv),
          b(Dl),
          b(o_),
          b(ue),
          b(bn),
          b(J),
          b(El),
          b(Mn, 8),
        );
      };
      static ɵprov = E({ token: e, factory: e.ɵfac });
    }
    return e;
  })(),
  Ho = class {
    eventManager;
    doc;
    ngZone;
    platformIsServer;
    tracingService;
    data = Object.create(null);
    throwOnSyntheticProps = !0;
    constructor(t, n, r, o, i) {
      ((this.eventManager = t),
        (this.doc = n),
        (this.ngZone = r),
        (this.platformIsServer = o),
        (this.tracingService = i));
    }
    destroy() {}
    destroyNode = null;
    createElement(t, n) {
      return n
        ? this.doc.createElementNS(Nd[n] || n, t)
        : this.doc.createElement(t);
    }
    createComment(t) {
      return this.doc.createComment(t);
    }
    createText(t) {
      return this.doc.createTextNode(t);
    }
    appendChild(t, n) {
      (wv(t) ? t.content : t).appendChild(n);
    }
    insertBefore(t, n, r) {
      t && (wv(t) ? t.content : t).insertBefore(n, r);
    }
    removeChild(t, n) {
      n.remove();
    }
    selectRootElement(t, n) {
      let r = typeof t == "string" ? this.doc.querySelector(t) : t;
      if (!r) throw new v(-5104, !1);
      return (n || (r.textContent = ""), r);
    }
    parentNode(t) {
      return t.parentNode;
    }
    nextSibling(t) {
      return t.nextSibling;
    }
    setAttribute(t, n, r, o) {
      if (o) {
        n = o + ":" + n;
        let i = Nd[o];
        i ? t.setAttributeNS(i, n, r) : t.setAttribute(n, r);
      } else t.setAttribute(n, r);
    }
    removeAttribute(t, n, r) {
      if (r) {
        let o = Nd[r];
        o ? t.removeAttributeNS(o, n) : t.removeAttribute(`${r}:${n}`);
      } else t.removeAttribute(n);
    }
    addClass(t, n) {
      t.classList.add(n);
    }
    removeClass(t, n) {
      t.classList.remove(n);
    }
    setStyle(t, n, r, o) {
      o & (Je.DashCase | Je.Important)
        ? t.style.setProperty(n, r, o & Je.Important ? "important" : "")
        : (t.style[n] = r);
    }
    removeStyle(t, n, r) {
      r & Je.DashCase ? t.style.removeProperty(n) : (t.style[n] = "");
    }
    setProperty(t, n, r) {
      t != null && (t[n] = r);
    }
    setValue(t, n) {
      t.nodeValue = n;
    }
    listen(t, n, r, o) {
      if (
        typeof t == "string" &&
        ((t = Mr().getGlobalEventTarget(this.doc, t)), !t)
      )
        throw new v(5102, !1);
      let i = this.decoratePreventDefault(r);
      return (
        this.tracingService?.wrapEventListener &&
          (i = this.tracingService.wrapEventListener(t, n, i)),
        this.eventManager.addEventListener(t, n, i, o)
      );
    }
    decoratePreventDefault(t) {
      return (n) => {
        if (n === "__ngUnwrap__") return t;
        (this.platformIsServer ? this.ngZone.runGuarded(() => t(n)) : t(n)) ===
          !1 && n.preventDefault();
      };
    }
  };
function wv(e) {
  return e.tagName === "TEMPLATE" && e.content !== void 0;
}
var Pd = class extends Ho {
    sharedStylesHost;
    hostEl;
    shadowRoot;
    constructor(t, n, r, o, i, s, a, c, u) {
      (super(t, i, s, c, u),
        (this.sharedStylesHost = n),
        (this.hostEl = r),
        (this.shadowRoot = r.attachShadow({ mode: "open" })),
        this.sharedStylesHost.addHost(this.shadowRoot));
      let l = o.styles;
      l = Mv(o.id, l);
      for (let h of l) {
        let f = document.createElement("style");
        (a && f.setAttribute("nonce", a),
          (f.textContent = h),
          this.shadowRoot.appendChild(f));
      }
      let d = o.getExternalStyles?.();
      if (d)
        for (let h of d) {
          let f = kd(h, i);
          (a && f.setAttribute("nonce", a), this.shadowRoot.appendChild(f));
        }
    }
    nodeOrShadowRoot(t) {
      return t === this.hostEl ? this.shadowRoot : t;
    }
    appendChild(t, n) {
      return super.appendChild(this.nodeOrShadowRoot(t), n);
    }
    insertBefore(t, n, r) {
      return super.insertBefore(this.nodeOrShadowRoot(t), n, r);
    }
    removeChild(t, n) {
      return super.removeChild(null, n);
    }
    parentNode(t) {
      return this.nodeOrShadowRoot(super.parentNode(this.nodeOrShadowRoot(t)));
    }
    destroy() {
      this.sharedStylesHost.removeHost(this.shadowRoot);
    }
  },
  zo = class extends Ho {
    sharedStylesHost;
    removeStylesOnCompDestroy;
    styles;
    styleUrls;
    constructor(t, n, r, o, i, s, a, c, u) {
      (super(t, i, s, a, c),
        (this.sharedStylesHost = n),
        (this.removeStylesOnCompDestroy = o));
      let l = r.styles;
      ((this.styles = u ? Mv(u, l) : l),
        (this.styleUrls = r.getExternalStyles?.(u)));
    }
    applyStyles() {
      this.sharedStylesHost.addStyles(this.styles, this.styleUrls);
    }
    destroy() {
      this.removeStylesOnCompDestroy &&
        this.sharedStylesHost.removeStyles(this.styles, this.styleUrls);
    }
  },
  wa = class extends zo {
    contentAttr;
    hostAttr;
    constructor(t, n, r, o, i, s, a, c, u) {
      let l = o + "-" + r.id;
      (super(t, n, r, i, s, a, c, u, l),
        (this.contentAttr = i_(l)),
        (this.hostAttr = s_(l)));
    }
    applyToHost(t) {
      (this.applyStyles(), this.setAttribute(t, this.hostAttr, ""));
    }
    createElement(t, n) {
      let r = super.createElement(t, n);
      return (super.setAttribute(r, this.contentAttr, ""), r);
    }
  },
  a_ = (() => {
    class e extends Da {
      constructor(n) {
        super(n);
      }
      supports(n) {
        return !0;
      }
      addEventListener(n, r, o, i) {
        return (
          n.addEventListener(r, o, i),
          () => this.removeEventListener(n, r, o, i)
        );
      }
      removeEventListener(n, r, o, i) {
        return n.removeEventListener(r, o, i);
      }
      static ɵfac = function (r) {
        return new (r || e)(b(ue));
      };
      static ɵprov = E({ token: e, factory: e.ɵfac });
    }
    return e;
  })(),
  Ev = ["alt", "control", "meta", "shift"],
  c_ = {
    "\b": "Backspace",
    "	": "Tab",
    "\x7F": "Delete",
    "\x1B": "Escape",
    Del: "Delete",
    Esc: "Escape",
    Left: "ArrowLeft",
    Right: "ArrowRight",
    Up: "ArrowUp",
    Down: "ArrowDown",
    Menu: "ContextMenu",
    Scroll: "ScrollLock",
    Win: "OS",
  },
  u_ = {
    alt: (e) => e.altKey,
    control: (e) => e.ctrlKey,
    meta: (e) => e.metaKey,
    shift: (e) => e.shiftKey,
  },
  l_ = (() => {
    class e extends Da {
      constructor(n) {
        super(n);
      }
      supports(n) {
        return e.parseEventName(n) != null;
      }
      addEventListener(n, r, o, i) {
        let s = e.parseEventName(r),
          a = e.eventCallback(s.fullKey, o, this.manager.getZone());
        return this.manager
          .getZone()
          .runOutsideAngular(() => Mr().onAndCancel(n, s.domEventName, a, i));
      }
      static parseEventName(n) {
        let r = n.toLowerCase().split("."),
          o = r.shift();
        if (r.length === 0 || !(o === "keydown" || o === "keyup")) return null;
        let i = e._normalizeKey(r.pop()),
          s = "",
          a = r.indexOf("code");
        if (
          (a > -1 && (r.splice(a, 1), (s = "code.")),
          Ev.forEach((u) => {
            let l = r.indexOf(u);
            l > -1 && (r.splice(l, 1), (s += u + "."));
          }),
          (s += i),
          r.length != 0 || i.length === 0)
        )
          return null;
        let c = {};
        return ((c.domEventName = o), (c.fullKey = s), c);
      }
      static matchEventFullKeyCode(n, r) {
        let o = c_[n.key] || n.key,
          i = "";
        return (
          r.indexOf("code.") > -1 && ((o = n.code), (i = "code.")),
          o == null || !o
            ? !1
            : ((o = o.toLowerCase()),
              o === " " ? (o = "space") : o === "." && (o = "dot"),
              Ev.forEach((s) => {
                if (s !== o) {
                  let a = u_[s];
                  a(n) && (i += s + ".");
                }
              }),
              (i += o),
              i === r)
        );
      }
      static eventCallback(n, r, o) {
        return (i) => {
          e.matchEventFullKeyCode(i, n) && o.runGuarded(() => r(i));
        };
      }
      static _normalizeKey(n) {
        return n === "esc" ? "escape" : n;
      }
      static ɵfac = function (r) {
        return new (r || e)(b(ue));
      };
      static ɵprov = E({ token: e, factory: e.ɵfac });
    }
    return e;
  })();
function mj(e, t) {
  return Ly(D({ rootComponent: e }, d_(t)));
}
function d_(e) {
  return {
    appProviders: [...m_, ...(e?.providers ?? [])],
    platformProviders: g_,
  };
}
function f_() {
  Ad.makeCurrent();
}
function h_() {
  return new Ue();
}
function p_() {
  return (Ig(document), document);
}
var g_ = [
  { provide: bn, useValue: Id },
  { provide: wl, useValue: f_, multi: !0 },
  { provide: ue, useFactory: p_, deps: [] },
];
var m_ = [
  { provide: ks, useValue: "root" },
  { provide: Ue, useFactory: h_, deps: [] },
  { provide: Od, useClass: a_, multi: !0, deps: [ue] },
  { provide: Od, useClass: l_, multi: !0, deps: [ue] },
  Dv,
  Cv,
  Iv,
  { provide: ur, useExisting: Dv },
  { provide: Sr, useClass: XT, deps: [] },
  [],
];
var Sv = (() => {
  class e {
    _doc;
    constructor(n) {
      this._doc = n;
    }
    getTitle() {
      return this._doc.title;
    }
    setTitle(n) {
      this._doc.title = n || "";
    }
    static ɵfac = function (r) {
      return new (r || e)(b(ue));
    };
    static ɵprov = E({ token: e, factory: e.ɵfac, providedIn: "root" });
  }
  return e;
})();
var y_ = (() => {
    class e {
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵprov = E({
        token: e,
        factory: function (r) {
          let o = null;
          return (r ? (o = new (r || e)()) : (o = b(v_)), o);
        },
        providedIn: "root",
      });
    }
    return e;
  })(),
  v_ = (() => {
    class e extends y_ {
      _doc;
      constructor(n) {
        (super(), (this._doc = n));
      }
      sanitize(n, r) {
        if (r == null) return null;
        switch (n) {
          case $e.NONE:
            return r;
          case $e.HTML:
            return Mt(r, "HTML") ? xe(r) : Tl(this._doc, String(r)).toString();
          case $e.STYLE:
            return Mt(r, "Style") ? xe(r) : r;
          case $e.SCRIPT:
            if (Mt(r, "Script")) return xe(r);
            throw new v(5200, !1);
          case $e.URL:
            return Mt(r, "URL") ? xe(r) : Mo(String(r));
          case $e.RESOURCE_URL:
            if (Mt(r, "ResourceURL")) return xe(r);
            throw new v(5201, !1);
          default:
            throw new v(5202, !1);
        }
      }
      bypassSecurityTrustHtml(n) {
        return Ng(n);
      }
      bypassSecurityTrustStyle(n) {
        return Rg(n);
      }
      bypassSecurityTrustScript(n) {
        return Ag(n);
      }
      bypassSecurityTrustUrl(n) {
        return Og(n);
      }
      bypassSecurityTrustResourceUrl(n) {
        return kg(n);
      }
      static ɵfac = function (r) {
        return new (r || e)(b(ue));
      };
      static ɵprov = E({ token: e, factory: e.ɵfac, providedIn: "root" });
    }
    return e;
  })();
var k = "primary",
  ri = Symbol("RouteTitle"),
  Ud = class {
    params;
    constructor(t) {
      this.params = t || {};
    }
    has(t) {
      return Object.prototype.hasOwnProperty.call(this.params, t);
    }
    get(t) {
      if (this.has(t)) {
        let n = this.params[t];
        return Array.isArray(n) ? n[0] : n;
      }
      return null;
    }
    getAll(t) {
      if (this.has(t)) {
        let n = this.params[t];
        return Array.isArray(n) ? n : [n];
      }
      return [];
    }
    get keys() {
      return Object.keys(this.params);
    }
  };
function kr(e) {
  return new Ud(e);
}
function w_(e, t, n) {
  let r = n.path.split("/");
  if (
    r.length > e.length ||
    (n.pathMatch === "full" && (t.hasChildren() || r.length < e.length))
  )
    return null;
  let o = {};
  for (let i = 0; i < r.length; i++) {
    let s = r[i],
      a = e[i];
    if (s[0] === ":") o[s.substring(1)] = a;
    else if (s !== a.path) return null;
  }
  return { consumed: e.slice(0, r.length), posParams: o };
}
function E_(e, t) {
  if (e.length !== t.length) return !1;
  for (let n = 0; n < e.length; ++n) if (!ot(e[n], t[n])) return !1;
  return !0;
}
function ot(e, t) {
  let n = e ? $d(e) : void 0,
    r = t ? $d(t) : void 0;
  if (!n || !r || n.length != r.length) return !1;
  let o;
  for (let i = 0; i < n.length; i++)
    if (((o = n[i]), !Fv(e[o], t[o]))) return !1;
  return !0;
}
function $d(e) {
  return [...Object.keys(e), ...Object.getOwnPropertySymbols(e)];
}
function Fv(e, t) {
  if (Array.isArray(e) && Array.isArray(t)) {
    if (e.length !== t.length) return !1;
    let n = [...e].sort(),
      r = [...t].sort();
    return n.every((o, i) => r[i] === o);
  } else return e === t;
}
function Lv(e) {
  return e.length > 0 ? e[e.length - 1] : null;
}
function Qt(e) {
  return uc(e) ? e : Cr(e) ? G(Promise.resolve(e)) : S(e);
}
var I_ = { exact: Vv, subset: Bv },
  jv = { exact: C_, subset: b_, ignored: () => !0 };
function Tv(e, t, n) {
  return (
    I_[n.paths](e.root, t.root, n.matrixParams) &&
    jv[n.queryParams](e.queryParams, t.queryParams) &&
    !(n.fragment === "exact" && e.fragment !== t.fragment)
  );
}
function C_(e, t) {
  return ot(e, t);
}
function Vv(e, t, n) {
  if (
    !xn(e.segments, t.segments) ||
    !Ca(e.segments, t.segments, n) ||
    e.numberOfChildren !== t.numberOfChildren
  )
    return !1;
  for (let r in t.children)
    if (!e.children[r] || !Vv(e.children[r], t.children[r], n)) return !1;
  return !0;
}
function b_(e, t) {
  return (
    Object.keys(t).length <= Object.keys(e).length &&
    Object.keys(t).every((n) => Fv(e[n], t[n]))
  );
}
function Bv(e, t, n) {
  return Uv(e, t, t.segments, n);
}
function Uv(e, t, n, r) {
  if (e.segments.length > n.length) {
    let o = e.segments.slice(0, n.length);
    return !(!xn(o, n) || t.hasChildren() || !Ca(o, n, r));
  } else if (e.segments.length === n.length) {
    if (!xn(e.segments, n) || !Ca(e.segments, n, r)) return !1;
    for (let o in t.children)
      if (!e.children[o] || !Bv(e.children[o], t.children[o], r)) return !1;
    return !0;
  } else {
    let o = n.slice(0, e.segments.length),
      i = n.slice(e.segments.length);
    return !xn(e.segments, o) || !Ca(e.segments, o, r) || !e.children[k]
      ? !1
      : Uv(e.children[k], t, i, r);
  }
}
function Ca(e, t, n) {
  return t.every((r, o) => jv[n](e[o].parameters, r.parameters));
}
var Rt = class {
    root;
    queryParams;
    fragment;
    _queryParamMap;
    constructor(t = new B([], {}), n = {}, r = null) {
      ((this.root = t), (this.queryParams = n), (this.fragment = r));
    }
    get queryParamMap() {
      return (
        (this._queryParamMap ??= kr(this.queryParams)),
        this._queryParamMap
      );
    }
    toString() {
      return T_.serialize(this);
    }
  },
  B = class {
    segments;
    children;
    parent = null;
    constructor(t, n) {
      ((this.segments = t),
        (this.children = n),
        Object.values(n).forEach((r) => (r.parent = this)));
    }
    hasChildren() {
      return this.numberOfChildren > 0;
    }
    get numberOfChildren() {
      return Object.keys(this.children).length;
    }
    toString() {
      return ba(this);
    }
  },
  _n = class {
    path;
    parameters;
    _parameterMap;
    constructor(t, n) {
      ((this.path = t), (this.parameters = n));
    }
    get parameterMap() {
      return ((this._parameterMap ??= kr(this.parameters)), this._parameterMap);
    }
    toString() {
      return Hv(this);
    }
  };
function M_(e, t) {
  return xn(e, t) && e.every((n, r) => ot(n.parameters, t[r].parameters));
}
function xn(e, t) {
  return e.length !== t.length ? !1 : e.every((n, r) => n.path === t[r].path);
}
function S_(e, t) {
  let n = [];
  return (
    Object.entries(e.children).forEach(([r, o]) => {
      r === k && (n = n.concat(t(o, r)));
    }),
    Object.entries(e.children).forEach(([r, o]) => {
      r !== k && (n = n.concat(t(o, r)));
    }),
    n
  );
}
var Vr = (() => {
    class e {
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵprov = E({
        token: e,
        factory: () => new Pr(),
        providedIn: "root",
      });
    }
    return e;
  })(),
  Pr = class {
    parse(t) {
      let n = new zd(t);
      return new Rt(
        n.parseRootSegment(),
        n.parseQueryParams(),
        n.parseFragment(),
      );
    }
    serialize(t) {
      let n = `/${qo(t.root, !0)}`,
        r = N_(t.queryParams),
        o = typeof t.fragment == "string" ? `#${__(t.fragment)}` : "";
      return `${n}${r}${o}`;
    }
  },
  T_ = new Pr();
function ba(e) {
  return e.segments.map((t) => Hv(t)).join("/");
}
function qo(e, t) {
  if (!e.hasChildren()) return ba(e);
  if (t) {
    let n = e.children[k] ? qo(e.children[k], !1) : "",
      r = [];
    return (
      Object.entries(e.children).forEach(([o, i]) => {
        o !== k && r.push(`${o}:${qo(i, !1)}`);
      }),
      r.length > 0 ? `${n}(${r.join("//")})` : n
    );
  } else {
    let n = S_(e, (r, o) =>
      o === k ? [qo(e.children[k], !1)] : [`${o}:${qo(r, !1)}`],
    );
    return Object.keys(e.children).length === 1 && e.children[k] != null
      ? `${ba(e)}/${n[0]}`
      : `${ba(e)}/(${n.join("//")})`;
  }
}
function $v(e) {
  return encodeURIComponent(e)
    .replace(/%40/g, "@")
    .replace(/%3A/gi, ":")
    .replace(/%24/g, "$")
    .replace(/%2C/gi, ",");
}
function Ea(e) {
  return $v(e).replace(/%3B/gi, ";");
}
function __(e) {
  return encodeURI(e);
}
function Hd(e) {
  return $v(e)
    .replace(/\(/g, "%28")
    .replace(/\)/g, "%29")
    .replace(/%26/gi, "&");
}
function Ma(e) {
  return decodeURIComponent(e);
}
function _v(e) {
  return Ma(e.replace(/\+/g, "%20"));
}
function Hv(e) {
  return `${Hd(e.path)}${x_(e.parameters)}`;
}
function x_(e) {
  return Object.entries(e)
    .map(([t, n]) => `;${Hd(t)}=${Hd(n)}`)
    .join("");
}
function N_(e) {
  let t = Object.entries(e)
    .map(([n, r]) =>
      Array.isArray(r)
        ? r.map((o) => `${Ea(n)}=${Ea(o)}`).join("&")
        : `${Ea(n)}=${Ea(r)}`,
    )
    .filter((n) => n);
  return t.length ? `?${t.join("&")}` : "";
}
var R_ = /^[^\/()?;#]+/;
function Ld(e) {
  let t = e.match(R_);
  return t ? t[0] : "";
}
var A_ = /^[^\/()?;=#]+/;
function O_(e) {
  let t = e.match(A_);
  return t ? t[0] : "";
}
var k_ = /^[^=?&#]+/;
function P_(e) {
  let t = e.match(k_);
  return t ? t[0] : "";
}
var F_ = /^[^&#]+/;
function L_(e) {
  let t = e.match(F_);
  return t ? t[0] : "";
}
var zd = class {
  url;
  remaining;
  constructor(t) {
    ((this.url = t), (this.remaining = t));
  }
  parseRootSegment() {
    return (
      this.consumeOptional("/"),
      this.remaining === "" ||
      this.peekStartsWith("?") ||
      this.peekStartsWith("#")
        ? new B([], {})
        : new B([], this.parseChildren())
    );
  }
  parseQueryParams() {
    let t = {};
    if (this.consumeOptional("?"))
      do this.parseQueryParam(t);
      while (this.consumeOptional("&"));
    return t;
  }
  parseFragment() {
    return this.consumeOptional("#")
      ? decodeURIComponent(this.remaining)
      : null;
  }
  parseChildren() {
    if (this.remaining === "") return {};
    this.consumeOptional("/");
    let t = [];
    for (
      this.peekStartsWith("(") || t.push(this.parseSegment());
      this.peekStartsWith("/") &&
      !this.peekStartsWith("//") &&
      !this.peekStartsWith("/(");
    )
      (this.capture("/"), t.push(this.parseSegment()));
    let n = {};
    this.peekStartsWith("/(") &&
      (this.capture("/"), (n = this.parseParens(!0)));
    let r = {};
    return (
      this.peekStartsWith("(") && (r = this.parseParens(!1)),
      (t.length > 0 || Object.keys(n).length > 0) && (r[k] = new B(t, n)),
      r
    );
  }
  parseSegment() {
    let t = Ld(this.remaining);
    if (t === "" && this.peekStartsWith(";")) throw new v(4009, !1);
    return (this.capture(t), new _n(Ma(t), this.parseMatrixParams()));
  }
  parseMatrixParams() {
    let t = {};
    for (; this.consumeOptional(";"); ) this.parseParam(t);
    return t;
  }
  parseParam(t) {
    let n = O_(this.remaining);
    if (!n) return;
    this.capture(n);
    let r = "";
    if (this.consumeOptional("=")) {
      let o = Ld(this.remaining);
      o && ((r = o), this.capture(r));
    }
    t[Ma(n)] = Ma(r);
  }
  parseQueryParam(t) {
    let n = P_(this.remaining);
    if (!n) return;
    this.capture(n);
    let r = "";
    if (this.consumeOptional("=")) {
      let s = L_(this.remaining);
      s && ((r = s), this.capture(r));
    }
    let o = _v(n),
      i = _v(r);
    if (t.hasOwnProperty(o)) {
      let s = t[o];
      (Array.isArray(s) || ((s = [s]), (t[o] = s)), s.push(i));
    } else t[o] = i;
  }
  parseParens(t) {
    let n = {};
    for (
      this.capture("(");
      !this.consumeOptional(")") && this.remaining.length > 0;
    ) {
      let r = Ld(this.remaining),
        o = this.remaining[r.length];
      if (o !== "/" && o !== ")" && o !== ";") throw new v(4010, !1);
      let i;
      r.indexOf(":") > -1
        ? ((i = r.slice(0, r.indexOf(":"))), this.capture(i), this.capture(":"))
        : t && (i = k);
      let s = this.parseChildren();
      ((n[i] = Object.keys(s).length === 1 ? s[k] : new B([], s)),
        this.consumeOptional("//"));
    }
    return n;
  }
  peekStartsWith(t) {
    return this.remaining.startsWith(t);
  }
  consumeOptional(t) {
    return this.peekStartsWith(t)
      ? ((this.remaining = this.remaining.substring(t.length)), !0)
      : !1;
  }
  capture(t) {
    if (!this.consumeOptional(t)) throw new v(4011, !1);
  }
};
function zv(e) {
  return e.segments.length > 0 ? new B([], { [k]: e }) : e;
}
function qv(e) {
  let t = {};
  for (let [r, o] of Object.entries(e.children)) {
    let i = qv(o);
    if (r === k && i.segments.length === 0 && i.hasChildren())
      for (let [s, a] of Object.entries(i.children)) t[s] = a;
    else (i.segments.length > 0 || i.hasChildren()) && (t[r] = i);
  }
  let n = new B(e.segments, t);
  return j_(n);
}
function j_(e) {
  if (e.numberOfChildren === 1 && e.children[k]) {
    let t = e.children[k];
    return new B(e.segments.concat(t.segments), t.children);
  }
  return e;
}
function Nn(e) {
  return e instanceof Rt;
}
function V_(e, t, n = null, r = null) {
  let o = Gv(e);
  return Wv(o, t, n, r);
}
function Gv(e) {
  let t;
  function n(i) {
    let s = {};
    for (let c of i.children) {
      let u = n(c);
      s[c.outlet] = u;
    }
    let a = new B(i.url, s);
    return (i === e && (t = a), a);
  }
  let r = n(e.root),
    o = zv(r);
  return t ?? o;
}
function Wv(e, t, n, r) {
  let o = e;
  for (; o.parent; ) o = o.parent;
  if (t.length === 0) return jd(o, o, o, n, r);
  let i = B_(t);
  if (i.toRoot()) return jd(o, o, new B([], {}), n, r);
  let s = U_(i, o, e),
    a = s.processChildren
      ? Wo(s.segmentGroup, s.index, i.commands)
      : Yv(s.segmentGroup, s.index, i.commands);
  return jd(o, s.segmentGroup, a, n, r);
}
function Ta(e) {
  return typeof e == "object" && e != null && !e.outlets && !e.segmentPath;
}
function Yo(e) {
  return typeof e == "object" && e != null && e.outlets;
}
function jd(e, t, n, r, o) {
  let i = {};
  r &&
    Object.entries(r).forEach(([c, u]) => {
      i[c] = Array.isArray(u) ? u.map((l) => `${l}`) : `${u}`;
    });
  let s;
  e === t ? (s = n) : (s = Zv(e, t, n));
  let a = zv(qv(s));
  return new Rt(a, i, o);
}
function Zv(e, t, n) {
  let r = {};
  return (
    Object.entries(e.children).forEach(([o, i]) => {
      i === t ? (r[o] = n) : (r[o] = Zv(i, t, n));
    }),
    new B(e.segments, r)
  );
}
var _a = class {
  isAbsolute;
  numberOfDoubleDots;
  commands;
  constructor(t, n, r) {
    if (
      ((this.isAbsolute = t),
      (this.numberOfDoubleDots = n),
      (this.commands = r),
      t && r.length > 0 && Ta(r[0]))
    )
      throw new v(4003, !1);
    let o = r.find(Yo);
    if (o && o !== Lv(r)) throw new v(4004, !1);
  }
  toRoot() {
    return (
      this.isAbsolute && this.commands.length === 1 && this.commands[0] == "/"
    );
  }
};
function B_(e) {
  if (typeof e[0] == "string" && e.length === 1 && e[0] === "/")
    return new _a(!0, 0, e);
  let t = 0,
    n = !1,
    r = e.reduce((o, i, s) => {
      if (typeof i == "object" && i != null) {
        if (i.outlets) {
          let a = {};
          return (
            Object.entries(i.outlets).forEach(([c, u]) => {
              a[c] = typeof u == "string" ? u.split("/") : u;
            }),
            [...o, { outlets: a }]
          );
        }
        if (i.segmentPath) return [...o, i.segmentPath];
      }
      return typeof i != "string"
        ? [...o, i]
        : s === 0
          ? (i.split("/").forEach((a, c) => {
              (c == 0 && a === ".") ||
                (c == 0 && a === ""
                  ? (n = !0)
                  : a === ".."
                    ? t++
                    : a != "" && o.push(a));
            }),
            o)
          : [...o, i];
    }, []);
  return new _a(n, t, r);
}
var Rr = class {
  segmentGroup;
  processChildren;
  index;
  constructor(t, n, r) {
    ((this.segmentGroup = t), (this.processChildren = n), (this.index = r));
  }
};
function U_(e, t, n) {
  if (e.isAbsolute) return new Rr(t, !0, 0);
  if (!n) return new Rr(t, !1, NaN);
  if (n.parent === null) return new Rr(n, !0, 0);
  let r = Ta(e.commands[0]) ? 0 : 1,
    o = n.segments.length - 1 + r;
  return $_(n, o, e.numberOfDoubleDots);
}
function $_(e, t, n) {
  let r = e,
    o = t,
    i = n;
  for (; i > o; ) {
    if (((i -= o), (r = r.parent), !r)) throw new v(4005, !1);
    o = r.segments.length;
  }
  return new Rr(r, !1, o - i);
}
function H_(e) {
  return Yo(e[0]) ? e[0].outlets : { [k]: e };
}
function Yv(e, t, n) {
  if (((e ??= new B([], {})), e.segments.length === 0 && e.hasChildren()))
    return Wo(e, t, n);
  let r = z_(e, t, n),
    o = n.slice(r.commandIndex);
  if (r.match && r.pathIndex < e.segments.length) {
    let i = new B(e.segments.slice(0, r.pathIndex), {});
    return (
      (i.children[k] = new B(e.segments.slice(r.pathIndex), e.children)),
      Wo(i, 0, o)
    );
  } else
    return r.match && o.length === 0
      ? new B(e.segments, {})
      : r.match && !e.hasChildren()
        ? qd(e, t, n)
        : r.match
          ? Wo(e, 0, o)
          : qd(e, t, n);
}
function Wo(e, t, n) {
  if (n.length === 0) return new B(e.segments, {});
  {
    let r = H_(n),
      o = {};
    if (
      Object.keys(r).some((i) => i !== k) &&
      e.children[k] &&
      e.numberOfChildren === 1 &&
      e.children[k].segments.length === 0
    ) {
      let i = Wo(e.children[k], t, n);
      return new B(e.segments, i.children);
    }
    return (
      Object.entries(r).forEach(([i, s]) => {
        (typeof s == "string" && (s = [s]),
          s !== null && (o[i] = Yv(e.children[i], t, s)));
      }),
      Object.entries(e.children).forEach(([i, s]) => {
        r[i] === void 0 && (o[i] = s);
      }),
      new B(e.segments, o)
    );
  }
}
function z_(e, t, n) {
  let r = 0,
    o = t,
    i = { match: !1, pathIndex: 0, commandIndex: 0 };
  for (; o < e.segments.length; ) {
    if (r >= n.length) return i;
    let s = e.segments[o],
      a = n[r];
    if (Yo(a)) break;
    let c = `${a}`,
      u = r < n.length - 1 ? n[r + 1] : null;
    if (o > 0 && c === void 0) break;
    if (c && u && typeof u == "object" && u.outlets === void 0) {
      if (!Nv(c, u, s)) return i;
      r += 2;
    } else {
      if (!Nv(c, {}, s)) return i;
      r++;
    }
    o++;
  }
  return { match: !0, pathIndex: o, commandIndex: r };
}
function qd(e, t, n) {
  let r = e.segments.slice(0, t),
    o = 0;
  for (; o < n.length; ) {
    let i = n[o];
    if (Yo(i)) {
      let c = q_(i.outlets);
      return new B(r, c);
    }
    if (o === 0 && Ta(n[0])) {
      let c = e.segments[t];
      (r.push(new _n(c.path, xv(n[0]))), o++);
      continue;
    }
    let s = Yo(i) ? i.outlets[k] : `${i}`,
      a = o < n.length - 1 ? n[o + 1] : null;
    s && a && Ta(a)
      ? (r.push(new _n(s, xv(a))), (o += 2))
      : (r.push(new _n(s, {})), o++);
  }
  return new B(r, {});
}
function q_(e) {
  let t = {};
  return (
    Object.entries(e).forEach(([n, r]) => {
      (typeof r == "string" && (r = [r]),
        r !== null && (t[n] = qd(new B([], {}), 0, r)));
    }),
    t
  );
}
function xv(e) {
  let t = {};
  return (Object.entries(e).forEach(([n, r]) => (t[n] = `${r}`)), t);
}
function Nv(e, t, n) {
  return e == n.path && ot(t, n.parameters);
}
var Sa = "imperative",
  ie = (function (e) {
    return (
      (e[(e.NavigationStart = 0)] = "NavigationStart"),
      (e[(e.NavigationEnd = 1)] = "NavigationEnd"),
      (e[(e.NavigationCancel = 2)] = "NavigationCancel"),
      (e[(e.NavigationError = 3)] = "NavigationError"),
      (e[(e.RoutesRecognized = 4)] = "RoutesRecognized"),
      (e[(e.ResolveStart = 5)] = "ResolveStart"),
      (e[(e.ResolveEnd = 6)] = "ResolveEnd"),
      (e[(e.GuardsCheckStart = 7)] = "GuardsCheckStart"),
      (e[(e.GuardsCheckEnd = 8)] = "GuardsCheckEnd"),
      (e[(e.RouteConfigLoadStart = 9)] = "RouteConfigLoadStart"),
      (e[(e.RouteConfigLoadEnd = 10)] = "RouteConfigLoadEnd"),
      (e[(e.ChildActivationStart = 11)] = "ChildActivationStart"),
      (e[(e.ChildActivationEnd = 12)] = "ChildActivationEnd"),
      (e[(e.ActivationStart = 13)] = "ActivationStart"),
      (e[(e.ActivationEnd = 14)] = "ActivationEnd"),
      (e[(e.Scroll = 15)] = "Scroll"),
      (e[(e.NavigationSkipped = 16)] = "NavigationSkipped"),
      e
    );
  })(ie || {}),
  Fe = class {
    id;
    url;
    constructor(t, n) {
      ((this.id = t), (this.url = n));
    }
  },
  Fr = class extends Fe {
    type = ie.NavigationStart;
    navigationTrigger;
    restoredState;
    constructor(t, n, r = "imperative", o = null) {
      (super(t, n), (this.navigationTrigger = r), (this.restoredState = o));
    }
    toString() {
      return `NavigationStart(id: ${this.id}, url: '${this.url}')`;
    }
  },
  it = class extends Fe {
    urlAfterRedirects;
    type = ie.NavigationEnd;
    constructor(t, n, r) {
      (super(t, n), (this.urlAfterRedirects = r));
    }
    toString() {
      return `NavigationEnd(id: ${this.id}, url: '${this.url}', urlAfterRedirects: '${this.urlAfterRedirects}')`;
    }
  },
  Re = (function (e) {
    return (
      (e[(e.Redirect = 0)] = "Redirect"),
      (e[(e.SupersededByNewNavigation = 1)] = "SupersededByNewNavigation"),
      (e[(e.NoDataFromResolver = 2)] = "NoDataFromResolver"),
      (e[(e.GuardRejected = 3)] = "GuardRejected"),
      e
    );
  })(Re || {}),
  xa = (function (e) {
    return (
      (e[(e.IgnoredSameUrlNavigation = 0)] = "IgnoredSameUrlNavigation"),
      (e[(e.IgnoredByUrlHandlingStrategy = 1)] =
        "IgnoredByUrlHandlingStrategy"),
      e
    );
  })(xa || {}),
  Nt = class extends Fe {
    reason;
    code;
    type = ie.NavigationCancel;
    constructor(t, n, r, o) {
      (super(t, n), (this.reason = r), (this.code = o));
    }
    toString() {
      return `NavigationCancel(id: ${this.id}, url: '${this.url}')`;
    }
  },
  Wt = class extends Fe {
    reason;
    code;
    type = ie.NavigationSkipped;
    constructor(t, n, r, o) {
      (super(t, n), (this.reason = r), (this.code = o));
    }
  },
  Qo = class extends Fe {
    error;
    target;
    type = ie.NavigationError;
    constructor(t, n, r, o) {
      (super(t, n), (this.error = r), (this.target = o));
    }
    toString() {
      return `NavigationError(id: ${this.id}, url: '${this.url}', error: ${this.error})`;
    }
  },
  Na = class extends Fe {
    urlAfterRedirects;
    state;
    type = ie.RoutesRecognized;
    constructor(t, n, r, o) {
      (super(t, n), (this.urlAfterRedirects = r), (this.state = o));
    }
    toString() {
      return `RoutesRecognized(id: ${this.id}, url: '${this.url}', urlAfterRedirects: '${this.urlAfterRedirects}', state: ${this.state})`;
    }
  },
  Gd = class extends Fe {
    urlAfterRedirects;
    state;
    type = ie.GuardsCheckStart;
    constructor(t, n, r, o) {
      (super(t, n), (this.urlAfterRedirects = r), (this.state = o));
    }
    toString() {
      return `GuardsCheckStart(id: ${this.id}, url: '${this.url}', urlAfterRedirects: '${this.urlAfterRedirects}', state: ${this.state})`;
    }
  },
  Wd = class extends Fe {
    urlAfterRedirects;
    state;
    shouldActivate;
    type = ie.GuardsCheckEnd;
    constructor(t, n, r, o, i) {
      (super(t, n),
        (this.urlAfterRedirects = r),
        (this.state = o),
        (this.shouldActivate = i));
    }
    toString() {
      return `GuardsCheckEnd(id: ${this.id}, url: '${this.url}', urlAfterRedirects: '${this.urlAfterRedirects}', state: ${this.state}, shouldActivate: ${this.shouldActivate})`;
    }
  },
  Zd = class extends Fe {
    urlAfterRedirects;
    state;
    type = ie.ResolveStart;
    constructor(t, n, r, o) {
      (super(t, n), (this.urlAfterRedirects = r), (this.state = o));
    }
    toString() {
      return `ResolveStart(id: ${this.id}, url: '${this.url}', urlAfterRedirects: '${this.urlAfterRedirects}', state: ${this.state})`;
    }
  },
  Yd = class extends Fe {
    urlAfterRedirects;
    state;
    type = ie.ResolveEnd;
    constructor(t, n, r, o) {
      (super(t, n), (this.urlAfterRedirects = r), (this.state = o));
    }
    toString() {
      return `ResolveEnd(id: ${this.id}, url: '${this.url}', urlAfterRedirects: '${this.urlAfterRedirects}', state: ${this.state})`;
    }
  },
  Qd = class {
    route;
    type = ie.RouteConfigLoadStart;
    constructor(t) {
      this.route = t;
    }
    toString() {
      return `RouteConfigLoadStart(path: ${this.route.path})`;
    }
  },
  Kd = class {
    route;
    type = ie.RouteConfigLoadEnd;
    constructor(t) {
      this.route = t;
    }
    toString() {
      return `RouteConfigLoadEnd(path: ${this.route.path})`;
    }
  },
  Jd = class {
    snapshot;
    type = ie.ChildActivationStart;
    constructor(t) {
      this.snapshot = t;
    }
    toString() {
      return `ChildActivationStart(path: '${(this.snapshot.routeConfig && this.snapshot.routeConfig.path) || ""}')`;
    }
  },
  Xd = class {
    snapshot;
    type = ie.ChildActivationEnd;
    constructor(t) {
      this.snapshot = t;
    }
    toString() {
      return `ChildActivationEnd(path: '${(this.snapshot.routeConfig && this.snapshot.routeConfig.path) || ""}')`;
    }
  },
  ef = class {
    snapshot;
    type = ie.ActivationStart;
    constructor(t) {
      this.snapshot = t;
    }
    toString() {
      return `ActivationStart(path: '${(this.snapshot.routeConfig && this.snapshot.routeConfig.path) || ""}')`;
    }
  },
  tf = class {
    snapshot;
    type = ie.ActivationEnd;
    constructor(t) {
      this.snapshot = t;
    }
    toString() {
      return `ActivationEnd(path: '${(this.snapshot.routeConfig && this.snapshot.routeConfig.path) || ""}')`;
    }
  },
  Ra = class {
    routerEvent;
    position;
    anchor;
    type = ie.Scroll;
    constructor(t, n, r) {
      ((this.routerEvent = t), (this.position = n), (this.anchor = r));
    }
    toString() {
      let t = this.position ? `${this.position[0]}, ${this.position[1]}` : null;
      return `Scroll(anchor: '${this.anchor}', position: '${t}')`;
    }
  },
  Ko = class {},
  Lr = class {
    url;
    navigationBehaviorOptions;
    constructor(t, n) {
      ((this.url = t), (this.navigationBehaviorOptions = n));
    }
  };
function G_(e, t) {
  return (
    e.providers &&
      !e._injector &&
      (e._injector = ta(e.providers, t, `Route: ${e.path}`)),
    e._injector ?? t
  );
}
function He(e) {
  return e.outlet || k;
}
function W_(e, t) {
  let n = e.filter((r) => He(r) === t);
  return (n.push(...e.filter((r) => He(r) !== t)), n);
}
function oi(e) {
  if (!e) return null;
  if (e.routeConfig?._injector) return e.routeConfig._injector;
  for (let t = e.parent; t; t = t.parent) {
    let n = t.routeConfig;
    if (n?._loadedInjector) return n._loadedInjector;
    if (n?._injector) return n._injector;
  }
  return null;
}
var nf = class {
    rootInjector;
    outlet = null;
    route = null;
    children;
    attachRef = null;
    get injector() {
      return oi(this.route?.snapshot) ?? this.rootInjector;
    }
    constructor(t) {
      ((this.rootInjector = t), (this.children = new ii(this.rootInjector)));
    }
  },
  ii = (() => {
    class e {
      rootInjector;
      contexts = new Map();
      constructor(n) {
        this.rootInjector = n;
      }
      onChildOutletCreated(n, r) {
        let o = this.getOrCreateContext(n);
        ((o.outlet = r), this.contexts.set(n, o));
      }
      onChildOutletDestroyed(n) {
        let r = this.getContext(n);
        r && ((r.outlet = null), (r.attachRef = null));
      }
      onOutletDeactivated() {
        let n = this.contexts;
        return ((this.contexts = new Map()), n);
      }
      onOutletReAttached(n) {
        this.contexts = n;
      }
      getOrCreateContext(n) {
        let r = this.getContext(n);
        return (
          r || ((r = new nf(this.rootInjector)), this.contexts.set(n, r)),
          r
        );
      }
      getContext(n) {
        return this.contexts.get(n) || null;
      }
      static ɵfac = function (r) {
        return new (r || e)(b(ye));
      };
      static ɵprov = E({ token: e, factory: e.ɵfac, providedIn: "root" });
    }
    return e;
  })(),
  Aa = class {
    _root;
    constructor(t) {
      this._root = t;
    }
    get root() {
      return this._root.value;
    }
    parent(t) {
      let n = this.pathFromRoot(t);
      return n.length > 1 ? n[n.length - 2] : null;
    }
    children(t) {
      let n = rf(t, this._root);
      return n ? n.children.map((r) => r.value) : [];
    }
    firstChild(t) {
      let n = rf(t, this._root);
      return n && n.children.length > 0 ? n.children[0].value : null;
    }
    siblings(t) {
      let n = of(t, this._root);
      return n.length < 2
        ? []
        : n[n.length - 2].children.map((o) => o.value).filter((o) => o !== t);
    }
    pathFromRoot(t) {
      return of(t, this._root).map((n) => n.value);
    }
  };
function rf(e, t) {
  if (e === t.value) return t;
  for (let n of t.children) {
    let r = rf(e, n);
    if (r) return r;
  }
  return null;
}
function of(e, t) {
  if (e === t.value) return [t];
  for (let n of t.children) {
    let r = of(e, n);
    if (r.length) return (r.unshift(t), r);
  }
  return [];
}
var Ne = class {
  value;
  children;
  constructor(t, n) {
    ((this.value = t), (this.children = n));
  }
  toString() {
    return `TreeNode(${this.value})`;
  }
};
function Nr(e) {
  let t = {};
  return (e && e.children.forEach((n) => (t[n.value.outlet] = n)), t);
}
var Oa = class extends Aa {
  snapshot;
  constructor(t, n) {
    (super(t), (this.snapshot = n), pf(this, t));
  }
  toString() {
    return this.snapshot.toString();
  }
};
function Qv(e) {
  let t = Z_(e),
    n = new se([new _n("", {})]),
    r = new se({}),
    o = new se({}),
    i = new se({}),
    s = new se(""),
    a = new Zt(n, r, i, s, o, k, e, t.root);
  return ((a.snapshot = t.root), new Oa(new Ne(a, []), t));
}
function Z_(e) {
  let t = {},
    n = {},
    r = {},
    o = "",
    i = new Ar([], t, r, o, n, k, e, null, {});
  return new Pa("", new Ne(i, []));
}
var Zt = class {
  urlSubject;
  paramsSubject;
  queryParamsSubject;
  fragmentSubject;
  dataSubject;
  outlet;
  component;
  snapshot;
  _futureSnapshot;
  _routerState;
  _paramMap;
  _queryParamMap;
  title;
  url;
  params;
  queryParams;
  fragment;
  data;
  constructor(t, n, r, o, i, s, a, c) {
    ((this.urlSubject = t),
      (this.paramsSubject = n),
      (this.queryParamsSubject = r),
      (this.fragmentSubject = o),
      (this.dataSubject = i),
      (this.outlet = s),
      (this.component = a),
      (this._futureSnapshot = c),
      (this.title = this.dataSubject?.pipe(O((u) => u[ri])) ?? S(void 0)),
      (this.url = t),
      (this.params = n),
      (this.queryParams = r),
      (this.fragment = o),
      (this.data = i));
  }
  get routeConfig() {
    return this._futureSnapshot.routeConfig;
  }
  get root() {
    return this._routerState.root;
  }
  get parent() {
    return this._routerState.parent(this);
  }
  get firstChild() {
    return this._routerState.firstChild(this);
  }
  get children() {
    return this._routerState.children(this);
  }
  get pathFromRoot() {
    return this._routerState.pathFromRoot(this);
  }
  get paramMap() {
    return (
      (this._paramMap ??= this.params.pipe(O((t) => kr(t)))),
      this._paramMap
    );
  }
  get queryParamMap() {
    return (
      (this._queryParamMap ??= this.queryParams.pipe(O((t) => kr(t)))),
      this._queryParamMap
    );
  }
  toString() {
    return this.snapshot
      ? this.snapshot.toString()
      : `Future(${this._futureSnapshot})`;
  }
};
function ka(e, t, n = "emptyOnly") {
  let r,
    { routeConfig: o } = e;
  return (
    t !== null &&
    (n === "always" ||
      o?.path === "" ||
      (!t.component && !t.routeConfig?.loadComponent))
      ? (r = {
          params: D(D({}, t.params), e.params),
          data: D(D({}, t.data), e.data),
          resolve: D(D(D(D({}, e.data), t.data), o?.data), e._resolvedData),
        })
      : (r = {
          params: D({}, e.params),
          data: D({}, e.data),
          resolve: D(D({}, e.data), e._resolvedData ?? {}),
        }),
    o && Jv(o) && (r.resolve[ri] = o.title),
    r
  );
}
var Ar = class {
    url;
    params;
    queryParams;
    fragment;
    data;
    outlet;
    component;
    routeConfig;
    _resolve;
    _resolvedData;
    _routerState;
    _paramMap;
    _queryParamMap;
    get title() {
      return this.data?.[ri];
    }
    constructor(t, n, r, o, i, s, a, c, u) {
      ((this.url = t),
        (this.params = n),
        (this.queryParams = r),
        (this.fragment = o),
        (this.data = i),
        (this.outlet = s),
        (this.component = a),
        (this.routeConfig = c),
        (this._resolve = u));
    }
    get root() {
      return this._routerState.root;
    }
    get parent() {
      return this._routerState.parent(this);
    }
    get firstChild() {
      return this._routerState.firstChild(this);
    }
    get children() {
      return this._routerState.children(this);
    }
    get pathFromRoot() {
      return this._routerState.pathFromRoot(this);
    }
    get paramMap() {
      return ((this._paramMap ??= kr(this.params)), this._paramMap);
    }
    get queryParamMap() {
      return (
        (this._queryParamMap ??= kr(this.queryParams)),
        this._queryParamMap
      );
    }
    toString() {
      let t = this.url.map((r) => r.toString()).join("/"),
        n = this.routeConfig ? this.routeConfig.path : "";
      return `Route(url:'${t}', path:'${n}')`;
    }
  },
  Pa = class extends Aa {
    url;
    constructor(t, n) {
      (super(n), (this.url = t), pf(this, n));
    }
    toString() {
      return Kv(this._root);
    }
  };
function pf(e, t) {
  ((t.value._routerState = e), t.children.forEach((n) => pf(e, n)));
}
function Kv(e) {
  let t = e.children.length > 0 ? ` { ${e.children.map(Kv).join(", ")} } ` : "";
  return `${e.value}${t}`;
}
function Vd(e) {
  if (e.snapshot) {
    let t = e.snapshot,
      n = e._futureSnapshot;
    ((e.snapshot = n),
      ot(t.queryParams, n.queryParams) ||
        e.queryParamsSubject.next(n.queryParams),
      t.fragment !== n.fragment && e.fragmentSubject.next(n.fragment),
      ot(t.params, n.params) || e.paramsSubject.next(n.params),
      E_(t.url, n.url) || e.urlSubject.next(n.url),
      ot(t.data, n.data) || e.dataSubject.next(n.data));
  } else
    ((e.snapshot = e._futureSnapshot),
      e.dataSubject.next(e._futureSnapshot.data));
}
function sf(e, t) {
  let n = ot(e.params, t.params) && M_(e.url, t.url),
    r = !e.parent != !t.parent;
  return n && !r && (!e.parent || sf(e.parent, t.parent));
}
function Jv(e) {
  return typeof e.title == "string" || e.title === null;
}
var Y_ = new I(""),
  Q_ = (() => {
    class e {
      activated = null;
      get activatedComponentRef() {
        return this.activated;
      }
      _activatedRoute = null;
      name = k;
      activateEvents = new _e();
      deactivateEvents = new _e();
      attachEvents = new _e();
      detachEvents = new _e();
      routerOutletData = bt(void 0);
      parentContexts = m(ii);
      location = m(nt);
      changeDetector = m(br);
      inputBinder = m(Va, { optional: !0 });
      supportsBindingToComponentInputs = !0;
      ngOnChanges(n) {
        if (n.name) {
          let { firstChange: r, previousValue: o } = n.name;
          if (r) return;
          (this.isTrackedInParentContexts(o) &&
            (this.deactivate(), this.parentContexts.onChildOutletDestroyed(o)),
            this.initializeOutletWithName());
        }
      }
      ngOnDestroy() {
        (this.isTrackedInParentContexts(this.name) &&
          this.parentContexts.onChildOutletDestroyed(this.name),
          this.inputBinder?.unsubscribeFromRouteData(this));
      }
      isTrackedInParentContexts(n) {
        return this.parentContexts.getContext(n)?.outlet === this;
      }
      ngOnInit() {
        this.initializeOutletWithName();
      }
      initializeOutletWithName() {
        if (
          (this.parentContexts.onChildOutletCreated(this.name, this),
          this.activated)
        )
          return;
        let n = this.parentContexts.getContext(this.name);
        n?.route &&
          (n.attachRef
            ? this.attach(n.attachRef, n.route)
            : this.activateWith(n.route, n.injector));
      }
      get isActivated() {
        return !!this.activated;
      }
      get component() {
        if (!this.activated) throw new v(4012, !1);
        return this.activated.instance;
      }
      get activatedRoute() {
        if (!this.activated) throw new v(4012, !1);
        return this._activatedRoute;
      }
      get activatedRouteData() {
        return this._activatedRoute ? this._activatedRoute.snapshot.data : {};
      }
      detach() {
        if (!this.activated) throw new v(4012, !1);
        this.location.detach();
        let n = this.activated;
        return (
          (this.activated = null),
          (this._activatedRoute = null),
          this.detachEvents.emit(n.instance),
          n
        );
      }
      attach(n, r) {
        ((this.activated = n),
          (this._activatedRoute = r),
          this.location.insert(n.hostView),
          this.inputBinder?.bindActivatedRouteToOutletComponent(this),
          this.attachEvents.emit(n.instance));
      }
      deactivate() {
        if (this.activated) {
          let n = this.component;
          (this.activated.destroy(),
            (this.activated = null),
            (this._activatedRoute = null),
            this.deactivateEvents.emit(n));
        }
      }
      activateWith(n, r) {
        if (this.isActivated) throw new v(4013, !1);
        this._activatedRoute = n;
        let o = this.location,
          s = n.snapshot.component,
          a = this.parentContexts.getOrCreateContext(this.name).children,
          c = new af(n, a, o.injector, this.routerOutletData);
        ((this.activated = o.createComponent(s, {
          index: o.length,
          injector: c,
          environmentInjector: r,
        })),
          this.changeDetector.markForCheck(),
          this.inputBinder?.bindActivatedRouteToOutletComponent(this),
          this.activateEvents.emit(this.activated.instance));
      }
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵdir = rt({
        type: e,
        selectors: [["router-outlet"]],
        inputs: { name: "name", routerOutletData: [1, "routerOutletData"] },
        outputs: {
          activateEvents: "activate",
          deactivateEvents: "deactivate",
          attachEvents: "attach",
          detachEvents: "detach",
        },
        exportAs: ["outlet"],
        features: [vr],
      });
    }
    return e;
  })(),
  af = class e {
    route;
    childContexts;
    parent;
    outletData;
    __ngOutletInjector(t) {
      return new e(this.route, this.childContexts, t, this.outletData);
    }
    constructor(t, n, r, o) {
      ((this.route = t),
        (this.childContexts = n),
        (this.parent = r),
        (this.outletData = o));
    }
    get(t, n) {
      return t === Zt
        ? this.route
        : t === ii
          ? this.childContexts
          : t === Y_
            ? this.outletData
            : this.parent.get(t, n);
    }
  },
  Va = new I(""),
  Rv = (() => {
    class e {
      outletDataSubscriptions = new Map();
      bindActivatedRouteToOutletComponent(n) {
        (this.unsubscribeFromRouteData(n), this.subscribeToRouteData(n));
      }
      unsubscribeFromRouteData(n) {
        (this.outletDataSubscriptions.get(n)?.unsubscribe(),
          this.outletDataSubscriptions.delete(n));
      }
      subscribeToRouteData(n) {
        let { activatedRoute: r } = n,
          o = Kr([r.queryParams, r.params, r.data])
            .pipe(
              me(
                ([i, s, a], c) => (
                  (a = D(D(D({}, i), s), a)),
                  c === 0 ? S(a) : Promise.resolve(a)
                ),
              ),
            )
            .subscribe((i) => {
              if (
                !n.isActivated ||
                !n.activatedComponentRef ||
                n.activatedRoute !== r ||
                r.component === null
              ) {
                this.unsubscribeFromRouteData(n);
                return;
              }
              let s = Uy(r.component);
              if (!s) {
                this.unsubscribeFromRouteData(n);
                return;
              }
              for (let { templateName: a } of s.inputs)
                n.activatedComponentRef.setInput(a, i[a]);
            });
        this.outletDataSubscriptions.set(n, o);
      }
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵprov = E({ token: e, factory: e.ɵfac });
    }
    return e;
  })();
function K_(e, t, n) {
  let r = Jo(e, t._root, n ? n._root : void 0);
  return new Oa(r, t);
}
function Jo(e, t, n) {
  if (n && e.shouldReuseRoute(t.value, n.value.snapshot)) {
    let r = n.value;
    r._futureSnapshot = t.value;
    let o = J_(e, t, n);
    return new Ne(r, o);
  } else {
    if (e.shouldAttach(t.value)) {
      let i = e.retrieve(t.value);
      if (i !== null) {
        let s = i.route;
        return (
          (s.value._futureSnapshot = t.value),
          (s.children = t.children.map((a) => Jo(e, a))),
          s
        );
      }
    }
    let r = X_(t.value),
      o = t.children.map((i) => Jo(e, i));
    return new Ne(r, o);
  }
}
function J_(e, t, n) {
  return t.children.map((r) => {
    for (let o of n.children)
      if (e.shouldReuseRoute(r.value, o.value.snapshot)) return Jo(e, r, o);
    return Jo(e, r);
  });
}
function X_(e) {
  return new Zt(
    new se(e.url),
    new se(e.params),
    new se(e.queryParams),
    new se(e.fragment),
    new se(e.data),
    e.outlet,
    e.component,
    e,
  );
}
var Xo = class {
    redirectTo;
    navigationBehaviorOptions;
    constructor(t, n) {
      ((this.redirectTo = t), (this.navigationBehaviorOptions = n));
    }
  },
  Xv = "ngNavigationCancelingError";
function Fa(e, t) {
  let { redirectTo: n, navigationBehaviorOptions: r } = Nn(t)
      ? { redirectTo: t, navigationBehaviorOptions: void 0 }
      : t,
    o = eD(!1, Re.Redirect);
  return ((o.url = n), (o.navigationBehaviorOptions = r), o);
}
function eD(e, t) {
  let n = new Error(`NavigationCancelingError: ${e || ""}`);
  return ((n[Xv] = !0), (n.cancellationCode = t), n);
}
function e0(e) {
  return tD(e) && Nn(e.url);
}
function tD(e) {
  return !!e && e[Xv];
}
var t0 = (e, t, n, r) =>
    O(
      (o) => (
        new cf(t, o.targetRouterState, o.currentRouterState, n, r).activate(e),
        o
      ),
    ),
  cf = class {
    routeReuseStrategy;
    futureState;
    currState;
    forwardEvent;
    inputBindingEnabled;
    constructor(t, n, r, o, i) {
      ((this.routeReuseStrategy = t),
        (this.futureState = n),
        (this.currState = r),
        (this.forwardEvent = o),
        (this.inputBindingEnabled = i));
    }
    activate(t) {
      let n = this.futureState._root,
        r = this.currState ? this.currState._root : null;
      (this.deactivateChildRoutes(n, r, t),
        Vd(this.futureState.root),
        this.activateChildRoutes(n, r, t));
    }
    deactivateChildRoutes(t, n, r) {
      let o = Nr(n);
      (t.children.forEach((i) => {
        let s = i.value.outlet;
        (this.deactivateRoutes(i, o[s], r), delete o[s]);
      }),
        Object.values(o).forEach((i) => {
          this.deactivateRouteAndItsChildren(i, r);
        }));
    }
    deactivateRoutes(t, n, r) {
      let o = t.value,
        i = n ? n.value : null;
      if (o === i) {
        if (o.component) {
          let s = r.getContext(o.outlet);
          s && this.deactivateChildRoutes(t, n, s.children);
        } else this.deactivateChildRoutes(t, n, r);
      } else i && this.deactivateRouteAndItsChildren(n, r);
    }
    deactivateRouteAndItsChildren(t, n) {
      t.value.component &&
      this.routeReuseStrategy.shouldDetach(t.value.snapshot)
        ? this.detachAndStoreRouteSubtree(t, n)
        : this.deactivateRouteAndOutlet(t, n);
    }
    detachAndStoreRouteSubtree(t, n) {
      let r = n.getContext(t.value.outlet),
        o = r && t.value.component ? r.children : n,
        i = Nr(t);
      for (let s of Object.values(i)) this.deactivateRouteAndItsChildren(s, o);
      if (r && r.outlet) {
        let s = r.outlet.detach(),
          a = r.children.onOutletDeactivated();
        this.routeReuseStrategy.store(t.value.snapshot, {
          componentRef: s,
          route: t,
          contexts: a,
        });
      }
    }
    deactivateRouteAndOutlet(t, n) {
      let r = n.getContext(t.value.outlet),
        o = r && t.value.component ? r.children : n,
        i = Nr(t);
      for (let s of Object.values(i)) this.deactivateRouteAndItsChildren(s, o);
      r &&
        (r.outlet && (r.outlet.deactivate(), r.children.onOutletDeactivated()),
        (r.attachRef = null),
        (r.route = null));
    }
    activateChildRoutes(t, n, r) {
      let o = Nr(n);
      (t.children.forEach((i) => {
        (this.activateRoutes(i, o[i.value.outlet], r),
          this.forwardEvent(new tf(i.value.snapshot)));
      }),
        t.children.length && this.forwardEvent(new Xd(t.value.snapshot)));
    }
    activateRoutes(t, n, r) {
      let o = t.value,
        i = n ? n.value : null;
      if ((Vd(o), o === i)) {
        if (o.component) {
          let s = r.getOrCreateContext(o.outlet);
          this.activateChildRoutes(t, n, s.children);
        } else this.activateChildRoutes(t, n, r);
      } else if (o.component) {
        let s = r.getOrCreateContext(o.outlet);
        if (this.routeReuseStrategy.shouldAttach(o.snapshot)) {
          let a = this.routeReuseStrategy.retrieve(o.snapshot);
          (this.routeReuseStrategy.store(o.snapshot, null),
            s.children.onOutletReAttached(a.contexts),
            (s.attachRef = a.componentRef),
            (s.route = a.route.value),
            s.outlet && s.outlet.attach(a.componentRef, a.route.value),
            Vd(a.route.value),
            this.activateChildRoutes(t, null, s.children));
        } else
          ((s.attachRef = null),
            (s.route = o),
            s.outlet && s.outlet.activateWith(o, s.injector),
            this.activateChildRoutes(t, null, s.children));
      } else this.activateChildRoutes(t, null, r);
    }
  },
  La = class {
    path;
    route;
    constructor(t) {
      ((this.path = t), (this.route = this.path[this.path.length - 1]));
    }
  },
  Or = class {
    component;
    route;
    constructor(t, n) {
      ((this.component = t), (this.route = n));
    }
  };
function n0(e, t, n) {
  let r = e._root,
    o = t ? t._root : null;
  return Go(r, o, n, [r.value]);
}
function r0(e) {
  let t = e.routeConfig ? e.routeConfig.canActivateChild : null;
  return !t || t.length === 0 ? null : { node: e, guards: t };
}
function Br(e, t) {
  let n = Symbol(),
    r = t.get(e, n);
  return r === n ? (typeof e == "function" && !lp(e) ? e : t.get(e)) : r;
}
function Go(
  e,
  t,
  n,
  r,
  o = { canDeactivateChecks: [], canActivateChecks: [] },
) {
  let i = Nr(t);
  return (
    e.children.forEach((s) => {
      (o0(s, i[s.value.outlet], n, r.concat([s.value]), o),
        delete i[s.value.outlet]);
    }),
    Object.entries(i).forEach(([s, a]) => Zo(a, n.getContext(s), o)),
    o
  );
}
function o0(
  e,
  t,
  n,
  r,
  o = { canDeactivateChecks: [], canActivateChecks: [] },
) {
  let i = e.value,
    s = t ? t.value : null,
    a = n ? n.getContext(e.value.outlet) : null;
  if (s && i.routeConfig === s.routeConfig) {
    let c = i0(s, i, i.routeConfig.runGuardsAndResolvers);
    (c
      ? o.canActivateChecks.push(new La(r))
      : ((i.data = s.data), (i._resolvedData = s._resolvedData)),
      i.component ? Go(e, t, a ? a.children : null, r, o) : Go(e, t, n, r, o),
      c &&
        a &&
        a.outlet &&
        a.outlet.isActivated &&
        o.canDeactivateChecks.push(new Or(a.outlet.component, s)));
  } else
    (s && Zo(t, a, o),
      o.canActivateChecks.push(new La(r)),
      i.component
        ? Go(e, null, a ? a.children : null, r, o)
        : Go(e, null, n, r, o));
  return o;
}
function i0(e, t, n) {
  if (typeof n == "function") return n(e, t);
  switch (n) {
    case "pathParamsChange":
      return !xn(e.url, t.url);
    case "pathParamsOrQueryParamsChange":
      return !xn(e.url, t.url) || !ot(e.queryParams, t.queryParams);
    case "always":
      return !0;
    case "paramsOrQueryParamsChange":
      return !sf(e, t) || !ot(e.queryParams, t.queryParams);
    case "paramsChange":
    default:
      return !sf(e, t);
  }
}
function Zo(e, t, n) {
  let r = Nr(e),
    o = e.value;
  (Object.entries(r).forEach(([i, s]) => {
    o.component
      ? t
        ? Zo(s, t.children.getContext(i), n)
        : Zo(s, null, n)
      : Zo(s, t, n);
  }),
    o.component
      ? t && t.outlet && t.outlet.isActivated
        ? n.canDeactivateChecks.push(new Or(t.outlet.component, o))
        : n.canDeactivateChecks.push(new Or(null, o))
      : n.canDeactivateChecks.push(new Or(null, o)));
}
function si(e) {
  return typeof e == "function";
}
function s0(e) {
  return typeof e == "boolean";
}
function a0(e) {
  return e && si(e.canLoad);
}
function c0(e) {
  return e && si(e.canActivate);
}
function u0(e) {
  return e && si(e.canActivateChild);
}
function l0(e) {
  return e && si(e.canDeactivate);
}
function d0(e) {
  return e && si(e.canMatch);
}
function nD(e) {
  return e instanceof je || e?.name === "EmptyError";
}
var Ia = Symbol("INITIAL_VALUE");
function jr() {
  return me((e) =>
    Kr(e.map((t) => t.pipe(Oe(1), Bi(Ia)))).pipe(
      O((t) => {
        for (let n of t)
          if (n !== !0) {
            if (n === Ia) return Ia;
            if (n === !1 || f0(n)) return n;
          }
        return !0;
      }),
      ge((t) => t !== Ia),
      Oe(1),
    ),
  );
}
function f0(e) {
  return Nn(e) || e instanceof Xo;
}
function h0(e, t) {
  return Z((n) => {
    let {
      targetSnapshot: r,
      currentSnapshot: o,
      guards: { canActivateChecks: i, canDeactivateChecks: s },
    } = n;
    return s.length === 0 && i.length === 0
      ? S(j(D({}, n), { guardsResult: !0 }))
      : p0(s, r, o, e).pipe(
          Z((a) => (a && s0(a) ? g0(r, i, e, t) : S(a))),
          O((a) => j(D({}, n), { guardsResult: a })),
        );
  });
}
function p0(e, t, n, r) {
  return G(e).pipe(
    Z((o) => w0(o.component, o.route, n, t, r)),
    qe((o) => o !== !0, !0),
  );
}
function g0(e, t, n, r) {
  return G(t).pipe(
    lt((o) =>
      Ft(
        y0(o.route.parent, r),
        m0(o.route, r),
        D0(e, o.path, n),
        v0(e, o.route, n),
      ),
    ),
    qe((o) => o !== !0, !0),
  );
}
function m0(e, t) {
  return (e !== null && t && t(new ef(e)), S(!0));
}
function y0(e, t) {
  return (e !== null && t && t(new Jd(e)), S(!0));
}
function v0(e, t, n) {
  let r = t.routeConfig ? t.routeConfig.canActivate : null;
  if (!r || r.length === 0) return S(!0);
  let o = r.map((i) =>
    ji(() => {
      let s = oi(t) ?? n,
        a = Br(i, s),
        c = c0(a) ? a.canActivate(t, e) : Me(s, () => a(t, e));
      return Qt(c).pipe(qe());
    }),
  );
  return S(o).pipe(jr());
}
function D0(e, t, n) {
  let r = t[t.length - 1],
    i = t
      .slice(0, t.length - 1)
      .reverse()
      .map((s) => r0(s))
      .filter((s) => s !== null)
      .map((s) =>
        ji(() => {
          let a = s.guards.map((c) => {
            let u = oi(s.node) ?? n,
              l = Br(c, u),
              d = u0(l) ? l.canActivateChild(r, e) : Me(u, () => l(r, e));
            return Qt(d).pipe(qe());
          });
          return S(a).pipe(jr());
        }),
      );
  return S(i).pipe(jr());
}
function w0(e, t, n, r, o) {
  let i = t && t.routeConfig ? t.routeConfig.canDeactivate : null;
  if (!i || i.length === 0) return S(!0);
  let s = i.map((a) => {
    let c = oi(t) ?? o,
      u = Br(a, c),
      l = l0(u) ? u.canDeactivate(e, t, n, r) : Me(c, () => u(e, t, n, r));
    return Qt(l).pipe(qe());
  });
  return S(s).pipe(jr());
}
function E0(e, t, n, r) {
  let o = t.canLoad;
  if (o === void 0 || o.length === 0) return S(!0);
  let i = o.map((s) => {
    let a = Br(s, e),
      c = a0(a) ? a.canLoad(t, n) : Me(e, () => a(t, n));
    return Qt(c);
  });
  return S(i).pipe(jr(), rD(r));
}
function rD(e) {
  return rc(
    oe((t) => {
      if (typeof t != "boolean") throw Fa(e, t);
    }),
    O((t) => t === !0),
  );
}
function I0(e, t, n, r) {
  let o = t.canMatch;
  if (!o || o.length === 0) return S(!0);
  let i = o.map((s) => {
    let a = Br(s, e),
      c = d0(a) ? a.canMatch(t, n) : Me(e, () => a(t, n));
    return Qt(c);
  });
  return S(i).pipe(jr(), rD(r));
}
var ei = class {
    segmentGroup;
    constructor(t) {
      this.segmentGroup = t || null;
    }
  },
  ti = class extends Error {
    urlTree;
    constructor(t) {
      (super(), (this.urlTree = t));
    }
  };
function xr(e) {
  return zn(new ei(e));
}
function C0(e) {
  return zn(new v(4e3, !1));
}
function b0(e) {
  return zn(eD(!1, Re.GuardRejected));
}
var uf = class {
    urlSerializer;
    urlTree;
    constructor(t, n) {
      ((this.urlSerializer = t), (this.urlTree = n));
    }
    lineralizeSegments(t, n) {
      let r = [],
        o = n.root;
      for (;;) {
        if (((r = r.concat(o.segments)), o.numberOfChildren === 0)) return S(r);
        if (o.numberOfChildren > 1 || !o.children[k])
          return C0(`${t.redirectTo}`);
        o = o.children[k];
      }
    }
    applyRedirectCommands(t, n, r, o, i) {
      if (typeof n != "string") {
        let a = n,
          {
            queryParams: c,
            fragment: u,
            routeConfig: l,
            url: d,
            outlet: h,
            params: f,
            data: p,
            title: g,
          } = o,
          y = Me(i, () =>
            a({
              params: f,
              data: p,
              queryParams: c,
              fragment: u,
              routeConfig: l,
              url: d,
              outlet: h,
              title: g,
            }),
          );
        if (y instanceof Rt) throw new ti(y);
        n = y;
      }
      let s = this.applyRedirectCreateUrlTree(
        n,
        this.urlSerializer.parse(n),
        t,
        r,
      );
      if (n[0] === "/") throw new ti(s);
      return s;
    }
    applyRedirectCreateUrlTree(t, n, r, o) {
      let i = this.createSegmentGroup(t, n.root, r, o);
      return new Rt(
        i,
        this.createQueryParams(n.queryParams, this.urlTree.queryParams),
        n.fragment,
      );
    }
    createQueryParams(t, n) {
      let r = {};
      return (
        Object.entries(t).forEach(([o, i]) => {
          if (typeof i == "string" && i[0] === ":") {
            let a = i.substring(1);
            r[o] = n[a];
          } else r[o] = i;
        }),
        r
      );
    }
    createSegmentGroup(t, n, r, o) {
      let i = this.createSegments(t, n.segments, r, o),
        s = {};
      return (
        Object.entries(n.children).forEach(([a, c]) => {
          s[a] = this.createSegmentGroup(t, c, r, o);
        }),
        new B(i, s)
      );
    }
    createSegments(t, n, r, o) {
      return n.map((i) =>
        i.path[0] === ":"
          ? this.findPosParam(t, i, o)
          : this.findOrReturn(i, r),
      );
    }
    findPosParam(t, n, r) {
      let o = r[n.path.substring(1)];
      if (!o) throw new v(4001, !1);
      return o;
    }
    findOrReturn(t, n) {
      let r = 0;
      for (let o of n) {
        if (o.path === t.path) return (n.splice(r), o);
        r++;
      }
      return t;
    }
  },
  lf = {
    matched: !1,
    consumedSegments: [],
    remainingSegments: [],
    parameters: {},
    positionalParamSegments: {},
  };
function M0(e, t, n, r, o) {
  let i = oD(e, t, n);
  return i.matched
    ? ((r = G_(t, r)),
      I0(r, t, n, o).pipe(O((s) => (s === !0 ? i : D({}, lf)))))
    : S(i);
}
function oD(e, t, n) {
  if (t.path === "**") return S0(n);
  if (t.path === "")
    return t.pathMatch === "full" && (e.hasChildren() || n.length > 0)
      ? D({}, lf)
      : {
          matched: !0,
          consumedSegments: [],
          remainingSegments: n,
          parameters: {},
          positionalParamSegments: {},
        };
  let o = (t.matcher || w_)(n, e, t);
  if (!o) return D({}, lf);
  let i = {};
  Object.entries(o.posParams ?? {}).forEach(([a, c]) => {
    i[a] = c.path;
  });
  let s =
    o.consumed.length > 0
      ? D(D({}, i), o.consumed[o.consumed.length - 1].parameters)
      : i;
  return {
    matched: !0,
    consumedSegments: o.consumed,
    remainingSegments: n.slice(o.consumed.length),
    parameters: s,
    positionalParamSegments: o.posParams ?? {},
  };
}
function S0(e) {
  return {
    matched: !0,
    parameters: e.length > 0 ? Lv(e).parameters : {},
    consumedSegments: e,
    remainingSegments: [],
    positionalParamSegments: {},
  };
}
function Av(e, t, n, r) {
  return n.length > 0 && x0(e, n, r)
    ? {
        segmentGroup: new B(t, _0(r, new B(n, e.children))),
        slicedSegments: [],
      }
    : n.length === 0 && N0(e, n, r)
      ? {
          segmentGroup: new B(e.segments, T0(e, n, r, e.children)),
          slicedSegments: n,
        }
      : { segmentGroup: new B(e.segments, e.children), slicedSegments: n };
}
function T0(e, t, n, r) {
  let o = {};
  for (let i of n)
    if (Ba(e, t, i) && !r[He(i)]) {
      let s = new B([], {});
      o[He(i)] = s;
    }
  return D(D({}, r), o);
}
function _0(e, t) {
  let n = {};
  n[k] = t;
  for (let r of e)
    if (r.path === "" && He(r) !== k) {
      let o = new B([], {});
      n[He(r)] = o;
    }
  return n;
}
function x0(e, t, n) {
  return n.some((r) => Ba(e, t, r) && He(r) !== k);
}
function N0(e, t, n) {
  return n.some((r) => Ba(e, t, r));
}
function Ba(e, t, n) {
  return (e.hasChildren() || t.length > 0) && n.pathMatch === "full"
    ? !1
    : n.path === "";
}
function R0(e, t, n) {
  return t.length === 0 && !e.children[n];
}
var df = class {};
function A0(e, t, n, r, o, i, s = "emptyOnly") {
  return new ff(e, t, n, r, o, s, i).recognize();
}
var O0 = 31,
  ff = class {
    injector;
    configLoader;
    rootComponentType;
    config;
    urlTree;
    paramsInheritanceStrategy;
    urlSerializer;
    applyRedirects;
    absoluteRedirectCount = 0;
    allowRedirects = !0;
    constructor(t, n, r, o, i, s, a) {
      ((this.injector = t),
        (this.configLoader = n),
        (this.rootComponentType = r),
        (this.config = o),
        (this.urlTree = i),
        (this.paramsInheritanceStrategy = s),
        (this.urlSerializer = a),
        (this.applyRedirects = new uf(this.urlSerializer, this.urlTree)));
    }
    noMatchError(t) {
      return new v(4002, `'${t.segmentGroup}'`);
    }
    recognize() {
      let t = Av(this.urlTree.root, [], [], this.config).segmentGroup;
      return this.match(t).pipe(
        O(({ children: n, rootSnapshot: r }) => {
          let o = new Ne(r, n),
            i = new Pa("", o),
            s = V_(r, [], this.urlTree.queryParams, this.urlTree.fragment);
          return (
            (s.queryParams = this.urlTree.queryParams),
            (i.url = this.urlSerializer.serialize(s)),
            { state: i, tree: s }
          );
        }),
      );
    }
    match(t) {
      let n = new Ar(
        [],
        Object.freeze({}),
        Object.freeze(D({}, this.urlTree.queryParams)),
        this.urlTree.fragment,
        Object.freeze({}),
        k,
        this.rootComponentType,
        null,
        {},
      );
      return this.processSegmentGroup(this.injector, this.config, t, k, n).pipe(
        O((r) => ({ children: r, rootSnapshot: n })),
        Lt((r) => {
          if (r instanceof ti)
            return ((this.urlTree = r.urlTree), this.match(r.urlTree.root));
          throw r instanceof ei ? this.noMatchError(r) : r;
        }),
      );
    }
    processSegmentGroup(t, n, r, o, i) {
      return r.segments.length === 0 && r.hasChildren()
        ? this.processChildren(t, n, r, i)
        : this.processSegment(t, n, r, r.segments, o, !0, i).pipe(
            O((s) => (s instanceof Ne ? [s] : [])),
          );
    }
    processChildren(t, n, r, o) {
      let i = [];
      for (let s of Object.keys(r.children))
        s === "primary" ? i.unshift(s) : i.push(s);
      return G(i).pipe(
        lt((s) => {
          let a = r.children[s],
            c = W_(n, s);
          return this.processSegmentGroup(t, c, a, s, o);
        }),
        hc((s, a) => (s.push(...a), s)),
        jt(null),
        fc(),
        Z((s) => {
          if (s === null) return xr(r);
          let a = iD(s);
          return (k0(a), S(a));
        }),
      );
    }
    processSegment(t, n, r, o, i, s, a) {
      return G(n).pipe(
        lt((c) =>
          this.processSegmentAgainstRoute(
            c._injector ?? t,
            n,
            c,
            r,
            o,
            i,
            s,
            a,
          ).pipe(
            Lt((u) => {
              if (u instanceof ei) return S(null);
              throw u;
            }),
          ),
        ),
        qe((c) => !!c),
        Lt((c) => {
          if (nD(c)) return R0(r, o, i) ? S(new df()) : xr(r);
          throw c;
        }),
      );
    }
    processSegmentAgainstRoute(t, n, r, o, i, s, a, c) {
      return He(r) !== s && (s === k || !Ba(o, i, r))
        ? xr(o)
        : r.redirectTo === void 0
          ? this.matchSegmentAgainstRoute(t, o, r, i, s, c)
          : this.allowRedirects && a
            ? this.expandSegmentAgainstRouteUsingRedirect(t, o, n, r, i, s, c)
            : xr(o);
    }
    expandSegmentAgainstRouteUsingRedirect(t, n, r, o, i, s, a) {
      let {
        matched: c,
        parameters: u,
        consumedSegments: l,
        positionalParamSegments: d,
        remainingSegments: h,
      } = oD(n, o, i);
      if (!c) return xr(n);
      typeof o.redirectTo == "string" &&
        o.redirectTo[0] === "/" &&
        (this.absoluteRedirectCount++,
        this.absoluteRedirectCount > O0 && (this.allowRedirects = !1));
      let f = new Ar(
          i,
          u,
          Object.freeze(D({}, this.urlTree.queryParams)),
          this.urlTree.fragment,
          Ov(o),
          He(o),
          o.component ?? o._loadedComponent ?? null,
          o,
          kv(o),
        ),
        p = ka(f, a, this.paramsInheritanceStrategy);
      ((f.params = Object.freeze(p.params)), (f.data = Object.freeze(p.data)));
      let g = this.applyRedirects.applyRedirectCommands(
        l,
        o.redirectTo,
        d,
        f,
        t,
      );
      return this.applyRedirects
        .lineralizeSegments(o, g)
        .pipe(Z((y) => this.processSegment(t, r, n, y.concat(h), s, !1, a)));
    }
    matchSegmentAgainstRoute(t, n, r, o, i, s) {
      let a = M0(n, r, o, t, this.urlSerializer);
      return (
        r.path === "**" && (n.children = {}),
        a.pipe(
          me((c) =>
            c.matched
              ? ((t = r._injector ?? t),
                this.getChildConfig(t, r, o).pipe(
                  me(({ routes: u }) => {
                    let l = r._loadedInjector ?? t,
                      {
                        parameters: d,
                        consumedSegments: h,
                        remainingSegments: f,
                      } = c,
                      p = new Ar(
                        h,
                        d,
                        Object.freeze(D({}, this.urlTree.queryParams)),
                        this.urlTree.fragment,
                        Ov(r),
                        He(r),
                        r.component ?? r._loadedComponent ?? null,
                        r,
                        kv(r),
                      ),
                      g = ka(p, s, this.paramsInheritanceStrategy);
                    ((p.params = Object.freeze(g.params)),
                      (p.data = Object.freeze(g.data)));
                    let { segmentGroup: y, slicedSegments: w } = Av(n, h, f, u);
                    if (w.length === 0 && y.hasChildren())
                      return this.processChildren(l, u, y, p).pipe(
                        O((H) => new Ne(p, H)),
                      );
                    if (u.length === 0 && w.length === 0)
                      return S(new Ne(p, []));
                    let L = He(r) === i;
                    return this.processSegment(
                      l,
                      u,
                      y,
                      w,
                      L ? k : i,
                      !0,
                      p,
                    ).pipe(O((H) => new Ne(p, H instanceof Ne ? [H] : [])));
                  }),
                ))
              : xr(n),
          ),
        )
      );
    }
    getChildConfig(t, n, r) {
      return n.children
        ? S({ routes: n.children, injector: t })
        : n.loadChildren
          ? n._loadedRoutes !== void 0
            ? S({ routes: n._loadedRoutes, injector: n._loadedInjector })
            : E0(t, n, r, this.urlSerializer).pipe(
                Z((o) =>
                  o
                    ? this.configLoader.loadChildren(t, n).pipe(
                        oe((i) => {
                          ((n._loadedRoutes = i.routes),
                            (n._loadedInjector = i.injector));
                        }),
                      )
                    : b0(n),
                ),
              )
          : S({ routes: [], injector: t });
    }
  };
function k0(e) {
  e.sort((t, n) =>
    t.value.outlet === k
      ? -1
      : n.value.outlet === k
        ? 1
        : t.value.outlet.localeCompare(n.value.outlet),
  );
}
function P0(e) {
  let t = e.value.routeConfig;
  return t && t.path === "";
}
function iD(e) {
  let t = [],
    n = new Set();
  for (let r of e) {
    if (!P0(r)) {
      t.push(r);
      continue;
    }
    let o = t.find((i) => r.value.routeConfig === i.value.routeConfig);
    o !== void 0 ? (o.children.push(...r.children), n.add(o)) : t.push(r);
  }
  for (let r of n) {
    let o = iD(r.children);
    t.push(new Ne(r.value, o));
  }
  return t.filter((r) => !n.has(r));
}
function Ov(e) {
  return e.data || {};
}
function kv(e) {
  return e.resolve || {};
}
function F0(e, t, n, r, o, i) {
  return Z((s) =>
    A0(e, t, n, r, s.extractedUrl, o, i).pipe(
      O(({ state: a, tree: c }) =>
        j(D({}, s), { targetSnapshot: a, urlAfterRedirects: c }),
      ),
    ),
  );
}
function L0(e, t) {
  return Z((n) => {
    let {
      targetSnapshot: r,
      guards: { canActivateChecks: o },
    } = n;
    if (!o.length) return S(n);
    let i = new Set(o.map((c) => c.route)),
      s = new Set();
    for (let c of i) if (!s.has(c)) for (let u of sD(c)) s.add(u);
    let a = 0;
    return G(s).pipe(
      lt((c) =>
        i.has(c)
          ? j0(c, r, e, t)
          : ((c.data = ka(c, c.parent, e).resolve), S(void 0)),
      ),
      oe(() => a++),
      Wn(1),
      Z((c) => (a === s.size ? S(n) : le)),
    );
  });
}
function sD(e) {
  let t = e.children.map((n) => sD(n)).flat();
  return [e, ...t];
}
function j0(e, t, n, r) {
  let o = e.routeConfig,
    i = e._resolve;
  return (
    o?.title !== void 0 && !Jv(o) && (i[ri] = o.title),
    V0(i, e, t, r).pipe(
      O(
        (s) => (
          (e._resolvedData = s),
          (e.data = ka(e, e.parent, n).resolve),
          null
        ),
      ),
    )
  );
}
function V0(e, t, n, r) {
  let o = $d(e);
  if (o.length === 0) return S({});
  let i = {};
  return G(o).pipe(
    Z((s) =>
      B0(e[s], t, n, r).pipe(
        qe(),
        oe((a) => {
          if (a instanceof Xo) throw Fa(new Pr(), a);
          i[s] = a;
        }),
      ),
    ),
    Wn(1),
    O(() => i),
    Lt((s) => (nD(s) ? le : zn(s))),
  );
}
function B0(e, t, n, r) {
  let o = oi(t) ?? r,
    i = Br(e, o),
    s = i.resolve ? i.resolve(t, n) : Me(o, () => i(t, n));
  return Qt(s);
}
function Bd(e) {
  return me((t) => {
    let n = e(t);
    return n ? G(n).pipe(O(() => t)) : S(t);
  });
}
var aD = (() => {
    class e {
      buildTitle(n) {
        let r,
          o = n.root;
        for (; o !== void 0; )
          ((r = this.getResolvedTitleForRoute(o) ?? r),
            (o = o.children.find((i) => i.outlet === k)));
        return r;
      }
      getResolvedTitleForRoute(n) {
        return n.data[ri];
      }
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵprov = E({ token: e, factory: () => m(U0), providedIn: "root" });
    }
    return e;
  })(),
  U0 = (() => {
    class e extends aD {
      title;
      constructor(n) {
        (super(), (this.title = n));
      }
      updateTitle(n) {
        let r = this.buildTitle(n);
        r !== void 0 && this.title.setTitle(r);
      }
      static ɵfac = function (r) {
        return new (r || e)(b(Sv));
      };
      static ɵprov = E({ token: e, factory: e.ɵfac, providedIn: "root" });
    }
    return e;
  })(),
  ai = new I("", { providedIn: "root", factory: () => ({}) }),
  $0 = (() => {
    class e {
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵcmp = na({
        type: e,
        selectors: [["ng-component"]],
        exportAs: ["emptyRouterOutlet"],
        decls: 1,
        vars: 0,
        template: function (r, o) {
          r & 1 && od(0, "router-outlet");
        },
        dependencies: [Q_],
        encapsulation: 2,
      });
    }
    return e;
  })();
function gf(e) {
  let t = e.children && e.children.map(gf),
    n = t ? j(D({}, e), { children: t }) : D({}, e);
  return (
    !n.component &&
      !n.loadComponent &&
      (t || n.loadChildren) &&
      n.outlet &&
      n.outlet !== k &&
      (n.component = $0),
    n
  );
}
var ni = new I(""),
  mf = (() => {
    class e {
      componentLoaders = new WeakMap();
      childrenLoaders = new WeakMap();
      onLoadStartListener;
      onLoadEndListener;
      compiler = m(ca);
      loadComponent(n) {
        if (this.componentLoaders.get(n)) return this.componentLoaders.get(n);
        if (n._loadedComponent) return S(n._loadedComponent);
        this.onLoadStartListener && this.onLoadStartListener(n);
        let r = Qt(n.loadComponent()).pipe(
            O(cD),
            oe((i) => {
              (this.onLoadEndListener && this.onLoadEndListener(n),
                (n._loadedComponent = i));
            }),
            an(() => {
              this.componentLoaders.delete(n);
            }),
          ),
          o = new Bn(r, () => new X()).pipe(Vn());
        return (this.componentLoaders.set(n, o), o);
      }
      loadChildren(n, r) {
        if (this.childrenLoaders.get(r)) return this.childrenLoaders.get(r);
        if (r._loadedRoutes)
          return S({ routes: r._loadedRoutes, injector: r._loadedInjector });
        this.onLoadStartListener && this.onLoadStartListener(r);
        let i = H0(r, this.compiler, n, this.onLoadEndListener).pipe(
            an(() => {
              this.childrenLoaders.delete(r);
            }),
          ),
          s = new Bn(i, () => new X()).pipe(Vn());
        return (this.childrenLoaders.set(r, s), s);
      }
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵprov = E({ token: e, factory: e.ɵfac, providedIn: "root" });
    }
    return e;
  })();
function H0(e, t, n, r) {
  return Qt(e.loadChildren()).pipe(
    O(cD),
    Z((o) =>
      o instanceof Ql || Array.isArray(o) ? S(o) : G(t.compileModuleAsync(o)),
    ),
    O((o) => {
      r && r(e);
      let i,
        s,
        a = !1;
      return (
        Array.isArray(o)
          ? ((s = o), (a = !0))
          : ((i = o.create(n).injector),
            (s = i.get(ni, [], { optional: !0, self: !0 }).flat())),
        { routes: s.map(gf), injector: i }
      );
    }),
  );
}
function z0(e) {
  return e && typeof e == "object" && "default" in e;
}
function cD(e) {
  return z0(e) ? e.default : e;
}
var yf = (() => {
    class e {
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵprov = E({ token: e, factory: () => m(q0), providedIn: "root" });
    }
    return e;
  })(),
  q0 = (() => {
    class e {
      shouldProcessUrl(n) {
        return !0;
      }
      extract(n) {
        return n;
      }
      merge(n, r) {
        return n;
      }
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵprov = E({ token: e, factory: e.ɵfac, providedIn: "root" });
    }
    return e;
  })(),
  uD = new I(""),
  lD = new I("");
function G0(e, t, n) {
  let r = e.get(lD),
    o = e.get(ue);
  return e.get(J).runOutsideAngular(() => {
    if (!o.startViewTransition || r.skipNextTransition)
      return ((r.skipNextTransition = !1), new Promise((u) => setTimeout(u)));
    let i,
      s = new Promise((u) => {
        i = u;
      }),
      a = o.startViewTransition(() => (i(), W0(e))),
      { onViewTransitionCreated: c } = r;
    return (c && Me(e, () => c({ transition: a, from: t, to: n })), s);
  });
}
function W0(e) {
  return new Promise((t) => {
    bl({ read: () => setTimeout(t) }, { injector: e });
  });
}
var dD = new I(""),
  Ua = (() => {
    class e {
      currentNavigation = null;
      currentTransition = null;
      lastSuccessfulNavigation = null;
      events = new X();
      transitionAbortSubject = new X();
      configLoader = m(mf);
      environmentInjector = m(ye);
      destroyRef = m(It);
      urlSerializer = m(Vr);
      rootContexts = m(ii);
      location = m(Tr);
      inputBindingEnabled = m(Va, { optional: !0 }) !== null;
      titleStrategy = m(aD);
      options = m(ai, { optional: !0 }) || {};
      paramsInheritanceStrategy =
        this.options.paramsInheritanceStrategy || "emptyOnly";
      urlHandlingStrategy = m(yf);
      createViewTransition = m(uD, { optional: !0 });
      navigationErrorHandler = m(dD, { optional: !0 });
      navigationId = 0;
      get hasRequestedNavigation() {
        return this.navigationId !== 0;
      }
      transitions;
      afterPreactivation = () => S(void 0);
      rootComponentType = null;
      destroyed = !1;
      constructor() {
        let n = (o) => this.events.next(new Qd(o)),
          r = (o) => this.events.next(new Kd(o));
        ((this.configLoader.onLoadEndListener = r),
          (this.configLoader.onLoadStartListener = n),
          this.destroyRef.onDestroy(() => {
            this.destroyed = !0;
          }));
      }
      complete() {
        this.transitions?.complete();
      }
      handleNavigationRequest(n) {
        let r = ++this.navigationId;
        this.transitions?.next(
          j(D({}, n), {
            extractedUrl: this.urlHandlingStrategy.extract(n.rawUrl),
            targetSnapshot: null,
            targetRouterState: null,
            guards: { canActivateChecks: [], canDeactivateChecks: [] },
            guardsResult: null,
            id: r,
          }),
        );
      }
      setupNavigations(n) {
        return (
          (this.transitions = new se(null)),
          this.transitions.pipe(
            ge((r) => r !== null),
            me((r) => {
              let o = !1,
                i = !1;
              return S(r).pipe(
                me((s) => {
                  if (this.navigationId > r.id)
                    return (
                      this.cancelNavigationTransition(
                        r,
                        "",
                        Re.SupersededByNewNavigation,
                      ),
                      le
                    );
                  ((this.currentTransition = r),
                    (this.currentNavigation = {
                      id: s.id,
                      initialUrl: s.rawUrl,
                      extractedUrl: s.extractedUrl,
                      targetBrowserUrl:
                        typeof s.extras.browserUrl == "string"
                          ? this.urlSerializer.parse(s.extras.browserUrl)
                          : s.extras.browserUrl,
                      trigger: s.source,
                      extras: s.extras,
                      previousNavigation: this.lastSuccessfulNavigation
                        ? j(D({}, this.lastSuccessfulNavigation), {
                            previousNavigation: null,
                          })
                        : null,
                    }));
                  let a =
                      !n.navigated ||
                      this.isUpdatingInternalState() ||
                      this.isUpdatedBrowserUrl(),
                    c = s.extras.onSameUrlNavigation ?? n.onSameUrlNavigation;
                  if (!a && c !== "reload") {
                    let u = "";
                    return (
                      this.events.next(
                        new Wt(
                          s.id,
                          this.urlSerializer.serialize(s.rawUrl),
                          u,
                          xa.IgnoredSameUrlNavigation,
                        ),
                      ),
                      s.resolve(!1),
                      le
                    );
                  }
                  if (this.urlHandlingStrategy.shouldProcessUrl(s.rawUrl))
                    return S(s).pipe(
                      me(
                        (u) => (
                          this.events.next(
                            new Fr(
                              u.id,
                              this.urlSerializer.serialize(u.extractedUrl),
                              u.source,
                              u.restoredState,
                            ),
                          ),
                          u.id !== this.navigationId ? le : Promise.resolve(u)
                        ),
                      ),
                      F0(
                        this.environmentInjector,
                        this.configLoader,
                        this.rootComponentType,
                        n.config,
                        this.urlSerializer,
                        this.paramsInheritanceStrategy,
                      ),
                      oe((u) => {
                        ((r.targetSnapshot = u.targetSnapshot),
                          (r.urlAfterRedirects = u.urlAfterRedirects),
                          (this.currentNavigation = j(
                            D({}, this.currentNavigation),
                            { finalUrl: u.urlAfterRedirects },
                          )));
                        let l = new Na(
                          u.id,
                          this.urlSerializer.serialize(u.extractedUrl),
                          this.urlSerializer.serialize(u.urlAfterRedirects),
                          u.targetSnapshot,
                        );
                        this.events.next(l);
                      }),
                    );
                  if (
                    a &&
                    this.urlHandlingStrategy.shouldProcessUrl(s.currentRawUrl)
                  ) {
                    let {
                        id: u,
                        extractedUrl: l,
                        source: d,
                        restoredState: h,
                        extras: f,
                      } = s,
                      p = new Fr(u, this.urlSerializer.serialize(l), d, h);
                    this.events.next(p);
                    let g = Qv(this.rootComponentType).snapshot;
                    return (
                      (this.currentTransition = r =
                        j(D({}, s), {
                          targetSnapshot: g,
                          urlAfterRedirects: l,
                          extras: j(D({}, f), {
                            skipLocationChange: !1,
                            replaceUrl: !1,
                          }),
                        })),
                      (this.currentNavigation.finalUrl = l),
                      S(r)
                    );
                  } else {
                    let u = "";
                    return (
                      this.events.next(
                        new Wt(
                          s.id,
                          this.urlSerializer.serialize(s.extractedUrl),
                          u,
                          xa.IgnoredByUrlHandlingStrategy,
                        ),
                      ),
                      s.resolve(!1),
                      le
                    );
                  }
                }),
                oe((s) => {
                  let a = new Gd(
                    s.id,
                    this.urlSerializer.serialize(s.extractedUrl),
                    this.urlSerializer.serialize(s.urlAfterRedirects),
                    s.targetSnapshot,
                  );
                  this.events.next(a);
                }),
                O(
                  (s) => (
                    (this.currentTransition = r =
                      j(D({}, s), {
                        guards: n0(
                          s.targetSnapshot,
                          s.currentSnapshot,
                          this.rootContexts,
                        ),
                      })),
                    r
                  ),
                ),
                h0(this.environmentInjector, (s) => this.events.next(s)),
                oe((s) => {
                  if (
                    ((r.guardsResult = s.guardsResult),
                    s.guardsResult && typeof s.guardsResult != "boolean")
                  )
                    throw Fa(this.urlSerializer, s.guardsResult);
                  let a = new Wd(
                    s.id,
                    this.urlSerializer.serialize(s.extractedUrl),
                    this.urlSerializer.serialize(s.urlAfterRedirects),
                    s.targetSnapshot,
                    !!s.guardsResult,
                  );
                  this.events.next(a);
                }),
                ge((s) =>
                  s.guardsResult
                    ? !0
                    : (this.cancelNavigationTransition(s, "", Re.GuardRejected),
                      !1),
                ),
                Bd((s) => {
                  if (s.guards.canActivateChecks.length !== 0)
                    return S(s).pipe(
                      oe((a) => {
                        let c = new Zd(
                          a.id,
                          this.urlSerializer.serialize(a.extractedUrl),
                          this.urlSerializer.serialize(a.urlAfterRedirects),
                          a.targetSnapshot,
                        );
                        this.events.next(c);
                      }),
                      me((a) => {
                        let c = !1;
                        return S(a).pipe(
                          L0(
                            this.paramsInheritanceStrategy,
                            this.environmentInjector,
                          ),
                          oe({
                            next: () => (c = !0),
                            complete: () => {
                              c ||
                                this.cancelNavigationTransition(
                                  a,
                                  "",
                                  Re.NoDataFromResolver,
                                );
                            },
                          }),
                        );
                      }),
                      oe((a) => {
                        let c = new Yd(
                          a.id,
                          this.urlSerializer.serialize(a.extractedUrl),
                          this.urlSerializer.serialize(a.urlAfterRedirects),
                          a.targetSnapshot,
                        );
                        this.events.next(c);
                      }),
                    );
                }),
                Bd((s) => {
                  let a = (c) => {
                    let u = [];
                    c.routeConfig?.loadComponent &&
                      !c.routeConfig._loadedComponent &&
                      u.push(
                        this.configLoader.loadComponent(c.routeConfig).pipe(
                          oe((l) => {
                            c.component = l;
                          }),
                          O(() => {}),
                        ),
                      );
                    for (let l of c.children) u.push(...a(l));
                    return u;
                  };
                  return Kr(a(s.targetSnapshot.root)).pipe(jt(null), Oe(1));
                }),
                Bd(() => this.afterPreactivation()),
                me(() => {
                  let { currentSnapshot: s, targetSnapshot: a } = r,
                    c = this.createViewTransition?.(
                      this.environmentInjector,
                      s.root,
                      a.root,
                    );
                  return c ? G(c).pipe(O(() => r)) : S(r);
                }),
                O((s) => {
                  let a = K_(
                    n.routeReuseStrategy,
                    s.targetSnapshot,
                    s.currentRouterState,
                  );
                  return (
                    (this.currentTransition = r =
                      j(D({}, s), { targetRouterState: a })),
                    (this.currentNavigation.targetRouterState = a),
                    r
                  );
                }),
                oe(() => {
                  this.events.next(new Ko());
                }),
                t0(
                  this.rootContexts,
                  n.routeReuseStrategy,
                  (s) => this.events.next(s),
                  this.inputBindingEnabled,
                ),
                Oe(1),
                oe({
                  next: (s) => {
                    ((o = !0),
                      (this.lastSuccessfulNavigation = this.currentNavigation),
                      this.events.next(
                        new it(
                          s.id,
                          this.urlSerializer.serialize(s.extractedUrl),
                          this.urlSerializer.serialize(s.urlAfterRedirects),
                        ),
                      ),
                      this.titleStrategy?.updateTitle(
                        s.targetRouterState.snapshot,
                      ),
                      s.resolve(!0));
                  },
                  complete: () => {
                    o = !0;
                  },
                }),
                Ui(
                  this.transitionAbortSubject.pipe(
                    oe((s) => {
                      throw s;
                    }),
                  ),
                ),
                an(() => {
                  (!o &&
                    !i &&
                    this.cancelNavigationTransition(
                      r,
                      "",
                      Re.SupersededByNewNavigation,
                    ),
                    this.currentTransition?.id === r.id &&
                      ((this.currentNavigation = null),
                      (this.currentTransition = null)));
                }),
                Lt((s) => {
                  if (this.destroyed) return (r.resolve(!1), le);
                  if (((i = !0), tD(s)))
                    (this.events.next(
                      new Nt(
                        r.id,
                        this.urlSerializer.serialize(r.extractedUrl),
                        s.message,
                        s.cancellationCode,
                      ),
                    ),
                      e0(s)
                        ? this.events.next(
                            new Lr(s.url, s.navigationBehaviorOptions),
                          )
                        : r.resolve(!1));
                  else {
                    let a = new Qo(
                      r.id,
                      this.urlSerializer.serialize(r.extractedUrl),
                      s,
                      r.targetSnapshot ?? void 0,
                    );
                    try {
                      let c = Me(this.environmentInjector, () =>
                        this.navigationErrorHandler?.(a),
                      );
                      if (c instanceof Xo) {
                        let { message: u, cancellationCode: l } = Fa(
                          this.urlSerializer,
                          c,
                        );
                        (this.events.next(
                          new Nt(
                            r.id,
                            this.urlSerializer.serialize(r.extractedUrl),
                            u,
                            l,
                          ),
                        ),
                          this.events.next(
                            new Lr(c.redirectTo, c.navigationBehaviorOptions),
                          ));
                      } else throw (this.events.next(a), s);
                    } catch (c) {
                      this.options.resolveNavigationPromiseOnError
                        ? r.resolve(!1)
                        : r.reject(c);
                    }
                  }
                  return le;
                }),
              );
            }),
          )
        );
      }
      cancelNavigationTransition(n, r, o) {
        let i = new Nt(
          n.id,
          this.urlSerializer.serialize(n.extractedUrl),
          r,
          o,
        );
        (this.events.next(i), n.resolve(!1));
      }
      isUpdatingInternalState() {
        return (
          this.currentTransition?.extractedUrl.toString() !==
          this.currentTransition?.currentUrlTree.toString()
        );
      }
      isUpdatedBrowserUrl() {
        let n = this.urlHandlingStrategy.extract(
            this.urlSerializer.parse(this.location.path(!0)),
          ),
          r =
            this.currentNavigation?.targetBrowserUrl ??
            this.currentNavigation?.extractedUrl;
        return (
          n.toString() !== r?.toString() &&
          !this.currentNavigation?.extras.skipLocationChange
        );
      }
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵprov = E({ token: e, factory: e.ɵfac, providedIn: "root" });
    }
    return e;
  })();
function Z0(e) {
  return e !== Sa;
}
var Y0 = (() => {
    class e {
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵprov = E({ token: e, factory: () => m(Q0), providedIn: "root" });
    }
    return e;
  })(),
  hf = class {
    shouldDetach(t) {
      return !1;
    }
    store(t, n) {}
    shouldAttach(t) {
      return !1;
    }
    retrieve(t) {
      return null;
    }
    shouldReuseRoute(t, n) {
      return t.routeConfig === n.routeConfig;
    }
  },
  Q0 = (() => {
    class e extends hf {
      static ɵfac = (() => {
        let n;
        return function (o) {
          return (n || (n = pl(e)))(o || e);
        };
      })();
      static ɵprov = E({ token: e, factory: e.ɵfac, providedIn: "root" });
    }
    return e;
  })(),
  fD = (() => {
    class e {
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵprov = E({ token: e, factory: () => m(K0), providedIn: "root" });
    }
    return e;
  })(),
  K0 = (() => {
    class e extends fD {
      location = m(Tr);
      urlSerializer = m(Vr);
      options = m(ai, { optional: !0 }) || {};
      canceledNavigationResolution =
        this.options.canceledNavigationResolution || "replace";
      urlHandlingStrategy = m(yf);
      urlUpdateStrategy = this.options.urlUpdateStrategy || "deferred";
      currentUrlTree = new Rt();
      getCurrentUrlTree() {
        return this.currentUrlTree;
      }
      rawUrlTree = this.currentUrlTree;
      getRawUrlTree() {
        return this.rawUrlTree;
      }
      currentPageId = 0;
      lastSuccessfulId = -1;
      restoredState() {
        return this.location.getState();
      }
      get browserPageId() {
        return this.canceledNavigationResolution !== "computed"
          ? this.currentPageId
          : (this.restoredState()?.ɵrouterPageId ?? this.currentPageId);
      }
      routerState = Qv(null);
      getRouterState() {
        return this.routerState;
      }
      stateMemento = this.createStateMemento();
      createStateMemento() {
        return {
          rawUrlTree: this.rawUrlTree,
          currentUrlTree: this.currentUrlTree,
          routerState: this.routerState,
        };
      }
      registerNonRouterCurrentEntryChangeListener(n) {
        return this.location.subscribe((r) => {
          r.type === "popstate" && n(r.url, r.state);
        });
      }
      handleRouterEvent(n, r) {
        if (n instanceof Fr) this.stateMemento = this.createStateMemento();
        else if (n instanceof Wt) this.rawUrlTree = r.initialUrl;
        else if (n instanceof Na) {
          if (
            this.urlUpdateStrategy === "eager" &&
            !r.extras.skipLocationChange
          ) {
            let o = this.urlHandlingStrategy.merge(r.finalUrl, r.initialUrl);
            this.setBrowserUrl(r.targetBrowserUrl ?? o, r);
          }
        } else
          n instanceof Ko
            ? ((this.currentUrlTree = r.finalUrl),
              (this.rawUrlTree = this.urlHandlingStrategy.merge(
                r.finalUrl,
                r.initialUrl,
              )),
              (this.routerState = r.targetRouterState),
              this.urlUpdateStrategy === "deferred" &&
                !r.extras.skipLocationChange &&
                this.setBrowserUrl(r.targetBrowserUrl ?? this.rawUrlTree, r))
            : n instanceof Nt &&
                (n.code === Re.GuardRejected ||
                  n.code === Re.NoDataFromResolver)
              ? this.restoreHistory(r)
              : n instanceof Qo
                ? this.restoreHistory(r, !0)
                : n instanceof it &&
                  ((this.lastSuccessfulId = n.id),
                  (this.currentPageId = this.browserPageId));
      }
      setBrowserUrl(n, r) {
        let o = n instanceof Rt ? this.urlSerializer.serialize(n) : n;
        if (this.location.isCurrentPathEqualTo(o) || r.extras.replaceUrl) {
          let i = this.browserPageId,
            s = D(D({}, r.extras.state), this.generateNgRouterState(r.id, i));
          this.location.replaceState(o, "", s);
        } else {
          let i = D(
            D({}, r.extras.state),
            this.generateNgRouterState(r.id, this.browserPageId + 1),
          );
          this.location.go(o, "", i);
        }
      }
      restoreHistory(n, r = !1) {
        if (this.canceledNavigationResolution === "computed") {
          let o = this.browserPageId,
            i = this.currentPageId - o;
          i !== 0
            ? this.location.historyGo(i)
            : this.currentUrlTree === n.finalUrl &&
              i === 0 &&
              (this.resetState(n), this.resetUrlToCurrentUrlTree());
        } else
          this.canceledNavigationResolution === "replace" &&
            (r && this.resetState(n), this.resetUrlToCurrentUrlTree());
      }
      resetState(n) {
        ((this.routerState = this.stateMemento.routerState),
          (this.currentUrlTree = this.stateMemento.currentUrlTree),
          (this.rawUrlTree = this.urlHandlingStrategy.merge(
            this.currentUrlTree,
            n.finalUrl ?? this.rawUrlTree,
          )));
      }
      resetUrlToCurrentUrlTree() {
        this.location.replaceState(
          this.urlSerializer.serialize(this.rawUrlTree),
          "",
          this.generateNgRouterState(this.lastSuccessfulId, this.currentPageId),
        );
      }
      generateNgRouterState(n, r) {
        return this.canceledNavigationResolution === "computed"
          ? { navigationId: n, ɵrouterPageId: r }
          : { navigationId: n };
      }
      static ɵfac = (() => {
        let n;
        return function (o) {
          return (n || (n = pl(e)))(o || e);
        };
      })();
      static ɵprov = E({ token: e, factory: e.ɵfac, providedIn: "root" });
    }
    return e;
  })();
function hD(e, t) {
  e.events
    .pipe(
      ge(
        (n) =>
          n instanceof it ||
          n instanceof Nt ||
          n instanceof Qo ||
          n instanceof Wt,
      ),
      O((n) =>
        n instanceof it || n instanceof Wt
          ? 0
          : (
                n instanceof Nt
                  ? n.code === Re.Redirect ||
                    n.code === Re.SupersededByNewNavigation
                  : !1
              )
            ? 2
            : 1,
      ),
      ge((n) => n !== 2),
      Oe(1),
    )
    .subscribe(() => {
      t();
    });
}
var J0 = {
    paths: "exact",
    fragment: "ignored",
    matrixParams: "ignored",
    queryParams: "exact",
  },
  X0 = {
    paths: "subset",
    fragment: "ignored",
    matrixParams: "ignored",
    queryParams: "subset",
  },
  Yt = (() => {
    class e {
      get currentUrlTree() {
        return this.stateManager.getCurrentUrlTree();
      }
      get rawUrlTree() {
        return this.stateManager.getRawUrlTree();
      }
      disposed = !1;
      nonRouterCurrentEntryChangeSubscription;
      console = m(Jl);
      stateManager = m(fD);
      options = m(ai, { optional: !0 }) || {};
      pendingTasks = m(Ct);
      urlUpdateStrategy = this.options.urlUpdateStrategy || "deferred";
      navigationTransitions = m(Ua);
      urlSerializer = m(Vr);
      location = m(Tr);
      urlHandlingStrategy = m(yf);
      _events = new X();
      get events() {
        return this._events;
      }
      get routerState() {
        return this.stateManager.getRouterState();
      }
      navigated = !1;
      routeReuseStrategy = m(Y0);
      onSameUrlNavigation = this.options.onSameUrlNavigation || "ignore";
      config = m(ni, { optional: !0 })?.flat() ?? [];
      componentInputBindingEnabled = !!m(Va, { optional: !0 });
      constructor() {
        (this.resetConfig(this.config),
          this.navigationTransitions.setupNavigations(this).subscribe({
            error: (n) => {
              this.console.warn(n);
            },
          }),
          this.subscribeToNavigationEvents());
      }
      eventsSubscription = new K();
      subscribeToNavigationEvents() {
        let n = this.navigationTransitions.events.subscribe((r) => {
          try {
            let o = this.navigationTransitions.currentTransition,
              i = this.navigationTransitions.currentNavigation;
            if (o !== null && i !== null) {
              if (
                (this.stateManager.handleRouterEvent(r, i),
                r instanceof Nt &&
                  r.code !== Re.Redirect &&
                  r.code !== Re.SupersededByNewNavigation)
              )
                this.navigated = !0;
              else if (r instanceof it) this.navigated = !0;
              else if (r instanceof Lr) {
                let s = r.navigationBehaviorOptions,
                  a = this.urlHandlingStrategy.merge(r.url, o.currentRawUrl),
                  c = D(
                    {
                      browserUrl: o.extras.browserUrl,
                      info: o.extras.info,
                      skipLocationChange: o.extras.skipLocationChange,
                      replaceUrl:
                        o.extras.replaceUrl ||
                        this.urlUpdateStrategy === "eager" ||
                        Z0(o.source),
                    },
                    s,
                  );
                this.scheduleNavigation(a, Sa, null, c, {
                  resolve: o.resolve,
                  reject: o.reject,
                  promise: o.promise,
                });
              }
            }
            tx(r) && this._events.next(r);
          } catch (o) {
            this.navigationTransitions.transitionAbortSubject.next(o);
          }
        });
        this.eventsSubscription.add(n);
      }
      resetRootComponentType(n) {
        ((this.routerState.root.component = n),
          (this.navigationTransitions.rootComponentType = n));
      }
      initialNavigation() {
        (this.setUpLocationChangeListener(),
          this.navigationTransitions.hasRequestedNavigation ||
            this.navigateToSyncWithBrowser(
              this.location.path(!0),
              Sa,
              this.stateManager.restoredState(),
            ));
      }
      setUpLocationChangeListener() {
        this.nonRouterCurrentEntryChangeSubscription ??=
          this.stateManager.registerNonRouterCurrentEntryChangeListener(
            (n, r) => {
              setTimeout(() => {
                this.navigateToSyncWithBrowser(n, "popstate", r);
              }, 0);
            },
          );
      }
      navigateToSyncWithBrowser(n, r, o) {
        let i = { replaceUrl: !0 },
          s = o?.navigationId ? o : null;
        if (o) {
          let c = D({}, o);
          (delete c.navigationId,
            delete c.ɵrouterPageId,
            Object.keys(c).length !== 0 && (i.state = c));
        }
        let a = this.parseUrl(n);
        this.scheduleNavigation(a, r, s, i);
      }
      get url() {
        return this.serializeUrl(this.currentUrlTree);
      }
      getCurrentNavigation() {
        return this.navigationTransitions.currentNavigation;
      }
      get lastSuccessfulNavigation() {
        return this.navigationTransitions.lastSuccessfulNavigation;
      }
      resetConfig(n) {
        ((this.config = n.map(gf)), (this.navigated = !1));
      }
      ngOnDestroy() {
        this.dispose();
      }
      dispose() {
        (this._events.unsubscribe(),
          this.navigationTransitions.complete(),
          this.nonRouterCurrentEntryChangeSubscription &&
            (this.nonRouterCurrentEntryChangeSubscription.unsubscribe(),
            (this.nonRouterCurrentEntryChangeSubscription = void 0)),
          (this.disposed = !0),
          this.eventsSubscription.unsubscribe());
      }
      createUrlTree(n, r = {}) {
        let {
            relativeTo: o,
            queryParams: i,
            fragment: s,
            queryParamsHandling: a,
            preserveFragment: c,
          } = r,
          u = c ? this.currentUrlTree.fragment : s,
          l = null;
        switch (a ?? this.options.defaultQueryParamsHandling) {
          case "merge":
            l = D(D({}, this.currentUrlTree.queryParams), i);
            break;
          case "preserve":
            l = this.currentUrlTree.queryParams;
            break;
          default:
            l = i || null;
        }
        l !== null && (l = this.removeEmptyProps(l));
        let d;
        try {
          let h = o ? o.snapshot : this.routerState.snapshot.root;
          d = Gv(h);
        } catch {
          ((typeof n[0] != "string" || n[0][0] !== "/") && (n = []),
            (d = this.currentUrlTree.root));
        }
        return Wv(d, n, l, u ?? null);
      }
      navigateByUrl(n, r = { skipLocationChange: !1 }) {
        let o = Nn(n) ? n : this.parseUrl(n),
          i = this.urlHandlingStrategy.merge(o, this.rawUrlTree);
        return this.scheduleNavigation(i, Sa, null, r);
      }
      navigate(n, r = { skipLocationChange: !1 }) {
        return (ex(n), this.navigateByUrl(this.createUrlTree(n, r), r));
      }
      serializeUrl(n) {
        return this.urlSerializer.serialize(n);
      }
      parseUrl(n) {
        try {
          return this.urlSerializer.parse(n);
        } catch {
          return this.urlSerializer.parse("/");
        }
      }
      isActive(n, r) {
        let o;
        if (
          (r === !0 ? (o = D({}, J0)) : r === !1 ? (o = D({}, X0)) : (o = r),
          Nn(n))
        )
          return Tv(this.currentUrlTree, n, o);
        let i = this.parseUrl(n);
        return Tv(this.currentUrlTree, i, o);
      }
      removeEmptyProps(n) {
        return Object.entries(n).reduce(
          (r, [o, i]) => (i != null && (r[o] = i), r),
          {},
        );
      }
      scheduleNavigation(n, r, o, i, s) {
        if (this.disposed) return Promise.resolve(!1);
        let a, c, u;
        s
          ? ((a = s.resolve), (c = s.reject), (u = s.promise))
          : (u = new Promise((d, h) => {
              ((a = d), (c = h));
            }));
        let l = this.pendingTasks.add();
        return (
          hD(this, () => {
            queueMicrotask(() => this.pendingTasks.remove(l));
          }),
          this.navigationTransitions.handleNavigationRequest({
            source: r,
            restoredState: o,
            currentUrlTree: this.currentUrlTree,
            currentRawUrl: this.currentUrlTree,
            rawUrl: n,
            extras: i,
            resolve: a,
            reject: c,
            promise: u,
            currentSnapshot: this.routerState.snapshot,
            currentRouterState: this.routerState,
          }),
          u.catch((d) => Promise.reject(d))
        );
      }
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵprov = E({ token: e, factory: e.ɵfac, providedIn: "root" });
    }
    return e;
  })();
function ex(e) {
  for (let t = 0; t < e.length; t++) if (e[t] == null) throw new v(4008, !1);
}
function tx(e) {
  return !(e instanceof Ko) && !(e instanceof Lr);
}
var Lj = (() => {
  class e {
    router;
    route;
    tabIndexAttribute;
    renderer;
    el;
    locationStrategy;
    href = null;
    target;
    queryParams;
    fragment;
    queryParamsHandling;
    state;
    info;
    relativeTo;
    isAnchorElement;
    subscription;
    onChanges = new X();
    constructor(n, r, o, i, s, a) {
      ((this.router = n),
        (this.route = r),
        (this.tabIndexAttribute = o),
        (this.renderer = i),
        (this.el = s),
        (this.locationStrategy = a));
      let c = s.nativeElement.tagName?.toLowerCase();
      ((this.isAnchorElement = c === "a" || c === "area"),
        this.isAnchorElement
          ? (this.subscription = n.events.subscribe((u) => {
              u instanceof it && this.updateHref();
            }))
          : this.setTabIndexIfNotOnNativeEl("0"));
    }
    preserveFragment = !1;
    skipLocationChange = !1;
    replaceUrl = !1;
    setTabIndexIfNotOnNativeEl(n) {
      this.tabIndexAttribute != null ||
        this.isAnchorElement ||
        this.applyAttributeValue("tabindex", n);
    }
    ngOnChanges(n) {
      (this.isAnchorElement && this.updateHref(), this.onChanges.next(this));
    }
    routerLinkInput = null;
    set routerLink(n) {
      n == null
        ? ((this.routerLinkInput = null), this.setTabIndexIfNotOnNativeEl(null))
        : (Nn(n)
            ? (this.routerLinkInput = n)
            : (this.routerLinkInput = Array.isArray(n) ? n : [n]),
          this.setTabIndexIfNotOnNativeEl("0"));
    }
    onClick(n, r, o, i, s) {
      let a = this.urlTree;
      if (
        a === null ||
        (this.isAnchorElement &&
          (n !== 0 ||
            r ||
            o ||
            i ||
            s ||
            (typeof this.target == "string" && this.target != "_self")))
      )
        return !0;
      let c = {
        skipLocationChange: this.skipLocationChange,
        replaceUrl: this.replaceUrl,
        state: this.state,
        info: this.info,
      };
      return (this.router.navigateByUrl(a, c), !this.isAnchorElement);
    }
    ngOnDestroy() {
      this.subscription?.unsubscribe();
    }
    updateHref() {
      let n = this.urlTree;
      this.href =
        n !== null && this.locationStrategy
          ? this.locationStrategy?.prepareExternalUrl(
              this.router.serializeUrl(n),
            )
          : null;
      let r =
        this.href === null
          ? null
          : Ug(this.href, this.el.nativeElement.tagName.toLowerCase(), "href");
      this.applyAttributeValue("href", r);
    }
    applyAttributeValue(n, r) {
      let o = this.renderer,
        i = this.el.nativeElement;
      r !== null ? o.setAttribute(i, n, r) : o.removeAttribute(i, n);
    }
    get urlTree() {
      return this.routerLinkInput === null
        ? null
        : Nn(this.routerLinkInput)
          ? this.routerLinkInput
          : this.router.createUrlTree(this.routerLinkInput, {
              relativeTo:
                this.relativeTo !== void 0 ? this.relativeTo : this.route,
              queryParams: this.queryParams,
              fragment: this.fragment,
              queryParamsHandling: this.queryParamsHandling,
              preserveFragment: this.preserveFragment,
            });
    }
    static ɵfac = function (r) {
      return new (r || e)(q(Yt), q(Zt), gl("tabindex"), q(Er), q(tt), q(xt));
    };
    static ɵdir = rt({
      type: e,
      selectors: [["", "routerLink", ""]],
      hostVars: 1,
      hostBindings: function (r, o) {
        (r & 1 &&
          id("click", function (s) {
            return o.onClick(
              s.button,
              s.ctrlKey,
              s.shiftKey,
              s.altKey,
              s.metaKey,
            );
          }),
          r & 2 && ed("target", o.target));
      },
      inputs: {
        target: "target",
        queryParams: "queryParams",
        fragment: "fragment",
        queryParamsHandling: "queryParamsHandling",
        state: "state",
        info: "info",
        relativeTo: "relativeTo",
        preserveFragment: [2, "preserveFragment", "preserveFragment", Po],
        skipLocationChange: [2, "skipLocationChange", "skipLocationChange", Po],
        replaceUrl: [2, "replaceUrl", "replaceUrl", Po],
        routerLink: "routerLink",
      },
      features: [vr],
    });
  }
  return e;
})();
var ja = class {};
var nx = (() => {
    class e {
      router;
      injector;
      preloadingStrategy;
      loader;
      subscription;
      constructor(n, r, o, i, s) {
        ((this.router = n),
          (this.injector = o),
          (this.preloadingStrategy = i),
          (this.loader = s));
      }
      setUpPreloading() {
        this.subscription = this.router.events
          .pipe(
            ge((n) => n instanceof it),
            lt(() => this.preload()),
          )
          .subscribe(() => {});
      }
      preload() {
        return this.processRoutes(this.injector, this.router.config);
      }
      ngOnDestroy() {
        this.subscription && this.subscription.unsubscribe();
      }
      processRoutes(n, r) {
        let o = [];
        for (let i of r) {
          i.providers &&
            !i._injector &&
            (i._injector = ta(i.providers, n, `Route: ${i.path}`));
          let s = i._injector ?? n,
            a = i._loadedInjector ?? s;
          (((i.loadChildren && !i._loadedRoutes && i.canLoad === void 0) ||
            (i.loadComponent && !i._loadedComponent)) &&
            o.push(this.preloadConfig(s, i)),
            (i.children || i._loadedRoutes) &&
              o.push(this.processRoutes(a, i.children ?? i._loadedRoutes)));
        }
        return G(o).pipe(Pt());
      }
      preloadConfig(n, r) {
        return this.preloadingStrategy.preload(r, () => {
          let o;
          r.loadChildren && r.canLoad === void 0
            ? (o = this.loader.loadChildren(n, r))
            : (o = S(null));
          let i = o.pipe(
            Z((s) =>
              s === null
                ? S(void 0)
                : ((r._loadedRoutes = s.routes),
                  (r._loadedInjector = s.injector),
                  this.processRoutes(s.injector ?? n, s.routes)),
            ),
          );
          if (r.loadComponent && !r._loadedComponent) {
            let s = this.loader.loadComponent(r);
            return G([i, s]).pipe(Pt());
          } else return i;
        });
      }
      static ɵfac = function (r) {
        return new (r || e)(b(Yt), b(ca), b(ye), b(ja), b(mf));
      };
      static ɵprov = E({ token: e, factory: e.ɵfac, providedIn: "root" });
    }
    return e;
  })(),
  vf = new I(""),
  pD = (() => {
    class e {
      urlSerializer;
      transitions;
      viewportScroller;
      zone;
      options;
      routerEventsSubscription;
      scrollEventsSubscription;
      lastId = 0;
      lastSource = "imperative";
      restoredId = 0;
      store = {};
      constructor(n, r, o, i, s = {}) {
        ((this.urlSerializer = n),
          (this.transitions = r),
          (this.viewportScroller = o),
          (this.zone = i),
          (this.options = s),
          (s.scrollPositionRestoration ||= "disabled"),
          (s.anchorScrolling ||= "disabled"));
      }
      init() {
        (this.options.scrollPositionRestoration !== "disabled" &&
          this.viewportScroller.setHistoryScrollRestoration("manual"),
          (this.routerEventsSubscription = this.createScrollEvents()),
          (this.scrollEventsSubscription = this.consumeScrollEvents()));
      }
      createScrollEvents() {
        return this.transitions.events.subscribe((n) => {
          n instanceof Fr
            ? ((this.store[this.lastId] =
                this.viewportScroller.getScrollPosition()),
              (this.lastSource = n.navigationTrigger),
              (this.restoredId = n.restoredState
                ? n.restoredState.navigationId
                : 0))
            : n instanceof it
              ? ((this.lastId = n.id),
                this.scheduleScrollEvent(
                  n,
                  this.urlSerializer.parse(n.urlAfterRedirects).fragment,
                ))
              : n instanceof Wt &&
                n.code === xa.IgnoredSameUrlNavigation &&
                ((this.lastSource = void 0),
                (this.restoredId = 0),
                this.scheduleScrollEvent(
                  n,
                  this.urlSerializer.parse(n.url).fragment,
                ));
        });
      }
      consumeScrollEvents() {
        return this.transitions.events.subscribe((n) => {
          n instanceof Ra &&
            (n.position
              ? this.options.scrollPositionRestoration === "top"
                ? this.viewportScroller.scrollToPosition([0, 0])
                : this.options.scrollPositionRestoration === "enabled" &&
                  this.viewportScroller.scrollToPosition(n.position)
              : n.anchor && this.options.anchorScrolling === "enabled"
                ? this.viewportScroller.scrollToAnchor(n.anchor)
                : this.options.scrollPositionRestoration !== "disabled" &&
                  this.viewportScroller.scrollToPosition([0, 0]));
        });
      }
      scheduleScrollEvent(n, r) {
        this.zone.runOutsideAngular(() => {
          setTimeout(() => {
            this.zone.run(() => {
              this.transitions.events.next(
                new Ra(
                  n,
                  this.lastSource === "popstate"
                    ? this.store[this.restoredId]
                    : null,
                  r,
                ),
              );
            });
          }, 0);
        });
      }
      ngOnDestroy() {
        (this.routerEventsSubscription?.unsubscribe(),
          this.scrollEventsSubscription?.unsubscribe());
      }
      static ɵfac = function (r) {
        Tm();
      };
      static ɵprov = E({ token: e, factory: e.ɵfac });
    }
    return e;
  })();
function jj(e, ...t) {
  return pr([
    { provide: ni, multi: !0, useValue: e },
    [],
    { provide: Zt, useFactory: gD, deps: [Yt] },
    { provide: Ro, multi: !0, useFactory: mD },
    t.map((n) => n.ɵproviders),
  ]);
}
function gD(e) {
  return e.routerState.root;
}
function Ur(e, t) {
  return { ɵkind: e, ɵproviders: t };
}
function Vj(e = {}) {
  return Ur(4, [
    {
      provide: vf,
      useFactory: () => {
        let n = m(bd),
          r = m(J),
          o = m(Ua),
          i = m(Vr);
        return new pD(i, o, n, r, e);
      },
    },
  ]);
}
function mD() {
  let e = m(ce);
  return (t) => {
    let n = e.get(vt);
    if (t !== n.components[0]) return;
    let r = e.get(Yt),
      o = e.get(yD);
    (e.get(Df) === 1 && r.initialNavigation(),
      e.get(vD, null, P.Optional)?.setUpPreloading(),
      e.get(vf, null, P.Optional)?.init(),
      r.resetRootComponentType(n.componentTypes[0]),
      o.closed || (o.next(), o.complete(), o.unsubscribe()));
  };
}
var yD = new I("", { factory: () => new X() }),
  Df = new I("", { providedIn: "root", factory: () => 1 });
function rx() {
  return Ur(2, [
    { provide: Df, useValue: 0 },
    {
      provide: oa,
      multi: !0,
      deps: [ce],
      useFactory: (t) => {
        let n = t.get(Jy, Promise.resolve());
        return () =>
          n.then(
            () =>
              new Promise((r) => {
                let o = t.get(Yt),
                  i = t.get(yD);
                (hD(o, () => {
                  r(!0);
                }),
                  (t.get(Ua).afterPreactivation = () => (
                    r(!0),
                    i.closed ? S(void 0) : i
                  )),
                  o.initialNavigation());
              }),
          );
      },
    },
  ]);
}
function ox() {
  return Ur(3, [
    {
      provide: oa,
      multi: !0,
      useFactory: () => {
        let t = m(Yt);
        return () => {
          t.setUpLocationChangeListener();
        };
      },
    },
    { provide: Df, useValue: 2 },
  ]);
}
var vD = new I("");
function ix(e) {
  return Ur(0, [
    { provide: vD, useExisting: nx },
    { provide: ja, useExisting: e },
  ]);
}
function sx() {
  return Ur(8, [Rv, { provide: Va, useExisting: Rv }]);
}
function ax(e) {
  let t = [
    { provide: uD, useValue: G0 },
    {
      provide: lD,
      useValue: D({ skipNextTransition: !!e?.skipInitialTransition }, e),
    },
  ];
  return Ur(9, t);
}
var cx = [
    Tr,
    { provide: Vr, useClass: Pr },
    Yt,
    ii,
    { provide: Zt, useFactory: gD, deps: [Yt] },
    mf,
    [],
  ],
  Bj = (() => {
    class e {
      constructor() {}
      static forRoot(n, r) {
        return {
          ngModule: e,
          providers: [
            cx,
            [],
            { provide: ni, multi: !0, useValue: n },
            [],
            r?.errorHandler ? { provide: dD, useValue: r.errorHandler } : [],
            { provide: ai, useValue: r || {} },
            r?.useHash ? lx() : dx(),
            ux(),
            r?.preloadingStrategy ? ix(r.preloadingStrategy).ɵproviders : [],
            r?.initialNavigation ? fx(r) : [],
            r?.bindToComponentInputs ? sx().ɵproviders : [],
            r?.enableViewTransitions ? ax().ɵproviders : [],
            hx(),
          ],
        };
      }
      static forChild(n) {
        return {
          ngModule: e,
          providers: [{ provide: ni, multi: !0, useValue: n }],
        };
      }
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵmod = Ir({ type: e });
      static ɵinj = hr({});
    }
    return e;
  })();
function ux() {
  return {
    provide: vf,
    useFactory: () => {
      let e = m(bd),
        t = m(J),
        n = m(ai),
        r = m(Ua),
        o = m(Vr);
      return (
        n.scrollOffset && e.setOffset(n.scrollOffset),
        new pD(o, r, e, t, n)
      );
    },
  };
}
function lx() {
  return { provide: xt, useClass: ev };
}
function dx() {
  return { provide: xt, useClass: Dd };
}
function fx(e) {
  return [
    e.initialNavigation === "disabled" ? ox().ɵproviders : [],
    e.initialNavigation === "enabledBlocking" ? rx().ɵproviders : [],
  ];
}
var Pv = new I("");
function hx() {
  return [
    { provide: Pv, useFactory: mD },
    { provide: Ro, multi: !0, useExisting: Pv },
  ];
}
var px = ["*"],
  Wj = (() => {
    class e {
      text = bt.required();
      variant = bt.required();
      textWrap = bt(!1);
      disabled = bt(!1);
      noBorder = bt(!1);
      type = bt("button");
      static ɵfac = function (r) {
        return new (r || e)();
      };
      static ɵcmp = na({
        type: e,
        selectors: [["pool-land-button"]],
        inputs: {
          text: [1, "text"],
          variant: [1, "variant"],
          textWrap: [1, "textWrap"],
          disabled: [1, "disabled"],
          noBorder: [1, "noBorder"],
          type: [1, "type"],
        },
        ngContentSelectors: px,
        decls: 4,
        vars: 7,
        consts: [
          [
            1,
            "flex",
            "w-full",
            "items-center",
            "justify-center",
            "gap-1",
            "rounded-full",
            "px-3",
            "py-2",
            "disabled:opacity-50",
            "xl:px-8",
            3,
            "ngClass",
            "disabled",
            "type",
          ],
          [1, "text-base", "font-medium", 3, "ngClass"],
        ],
        template: function (r, o) {
          (r & 1 &&
            (by(), Ao(0, "button", 0), My(1), Ao(2, "span", 1), Ty(3), sa()()),
            r & 2 &&
              (nd("no-border", o.noBorder()),
              ia("ngClass", o.variant())("disabled", o.disabled())(
                "type",
                o.type(),
              ),
              kl(2),
              ia("ngClass", o.textWrap() ? "text-wrap" : "text-nowrap"),
              kl(),
              sd(o.text())));
        },
        dependencies: [Ed, rv],
        styles: [
          '[_nghost-%COMP%]{display:inline-flex}button[_ngcontent-%COMP%]:not([disabled]):hover{filter:brightness(1.1) saturate(1.1)}.primary[_ngcontent-%COMP%]{--tw-border-opacity: 1;border-color:rgb(0 0 255 / var(--tw-border-opacity, 1));--tw-bg-opacity: 1;background-color:rgb(98 247 164 / var(--tw-bg-opacity, 1));--tw-text-opacity: 1;color:rgb(0 0 255 / var(--tw-text-opacity, 1));border:1px solid}.no-border[_ngcontent-%COMP%]{border:none!important}.primary-secondary-green[_ngcontent-%COMP%], .primary-secondary-green-with-blue-hover[_ngcontent-%COMP%]{--tw-border-opacity: 1;border-color:rgb(0 0 255 / var(--tw-border-opacity, 1));--tw-bg-opacity: 1;background-color:rgb(152 251 150 / var(--tw-bg-opacity, 1));--tw-text-opacity: 1;color:rgb(0 0 255 / var(--tw-text-opacity, 1));border:1px solid}.primary-secondary-green-with-blue-hover[_ngcontent-%COMP%]:hover{--tw-text-opacity: 1;color:rgb(152 251 150 / var(--tw-text-opacity, 1));--tw-bg-opacity: 1;background-color:rgb(0 0 255 / var(--tw-bg-opacity, 1))}.group[_ngcontent-%COMP%]:hover   .primary-secondary-green-with-blue-hover[_ngcontent-%COMP%]{--tw-text-opacity: 1;color:rgb(152 251 150 / var(--tw-text-opacity, 1))}.secondary[_ngcontent-%COMP%]{border-width:1px;border-style:solid;--tw-border-opacity: 1;border-color:rgb(255 255 255 / var(--tw-border-opacity, 1));--tw-bg-opacity: 1;background-color:rgb(3 21 131 / var(--tw-bg-opacity, 1));--tw-text-opacity: 1;color:rgb(152 251 150 / var(--tw-text-opacity, 1))}.tertiary[_ngcontent-%COMP%]{border-width:1px;border-style:solid;--tw-border-opacity: 1;border-color:rgb(98 247 164 / var(--tw-border-opacity, 1));background-color:#e7e7e74d;--tw-bg-opacity: .3;--tw-text-opacity: 1;color:rgb(98 247 164 / var(--tw-text-opacity, 1))}.basic[_ngcontent-%COMP%]{--tw-bg-opacity: 1;background-color:rgb(0 0 255 / var(--tw-bg-opacity, 1));--tw-text-opacity: 1;color:rgb(255 255 255 / var(--tw-text-opacity, 1))}.primary-full-width[_ngcontent-%COMP%]{--tw-border-opacity: 1;border-color:rgb(0 0 255 / var(--tw-border-opacity, 1));--tw-bg-opacity: 1;background-color:rgb(98 247 164 / var(--tw-bg-opacity, 1));--tw-text-opacity: 1;color:rgb(0 0 255 / var(--tw-text-opacity, 1));width:100%;justify-content:center;border:1px solid}.secondary-green-transparent[_ngcontent-%COMP%]{border-width:1px;--tw-border-opacity: 1;border-color:rgb(152 251 150 / var(--tw-border-opacity, 1));--tw-text-opacity: 1;color:rgb(152 251 150 / var(--tw-text-opacity, 1))}.animated-border[_ngcontent-%COMP%]{position:relative;background:transparent;--tw-text-opacity: 1;color:rgb(152 251 150 / var(--tw-text-opacity, 1))}.animated-border[_ngcontent-%COMP%]:before{content:"";position:absolute;inset:-2px;padding:2px;border-radius:9999px;background:linear-gradient(135deg,#00c853,#00c8531a,#00c853,#00c8531a,#00c853,#00c8531a,#00c853,#00c8531a,#00c853,#00c8531a,#00c853);background-size:400% 400%;-webkit-mask:linear-gradient(#fff 0 0) content-box,linear-gradient(#fff 0 0);-webkit-mask-composite:xor;mask:linear-gradient(#fff 0 0) content-box,linear-gradient(#fff 0 0);mask-composite:exclude;animation:_ngcontent-%COMP%_moveGradient 15s linear infinite}@keyframes _ngcontent-%COMP%_moveGradient{0%{background-position:0% 0%}50%{background-position:100% 100%}to{background-position:0% 0%}}',
        ],
        changeDetection: 0,
      });
    }
    return e;
  })();
export {
  K as a,
  F as b,
  X as c,
  se as d,
  ac as e,
  G as f,
  S as g,
  uc as h,
  LD as i,
  O as j,
  GD as k,
  lc as l,
  XD as m,
  ew as n,
  ge as o,
  tw as p,
  eh as q,
  nw as r,
  qe as s,
  Bi as t,
  me as u,
  Ui as v,
  ow as w,
  oe as x,
  iw as y,
  v as z,
  cp as A,
  E as B,
  hr as C,
  I as D,
  b as E,
  m as F,
  ye as G,
  Me as H,
  gr as I,
  vr as J,
  o1 as K,
  i1 as L,
  s1 as M,
  a1 as N,
  pl as O,
  ce as P,
  It as Q,
  _e as R,
  J as S,
  c1 as T,
  bt as U,
  tt as V,
  u1 as W,
  sr as X,
  Yc as Y,
  Dl as Z,
  bn as _,
  l1 as $,
  YE as aa,
  bl as ba,
  d1 as ca,
  pI as da,
  gI as ea,
  f1 as fa,
  kl as ga,
  Dn as ha,
  ur as ia,
  Er as ja,
  q as ka,
  nt as la,
  w1 as ma,
  E1 as na,
  I1 as oa,
  C1 as pa,
  En as qa,
  na as ra,
  Ir as sa,
  rt as ta,
  ra as ua,
  kb as va,
  S1 as wa,
  zb as xa,
  Cr as ya,
  vt as za,
  ed as Aa,
  ia as Ba,
  ny as Ca,
  nd as Da,
  T1 as Ea,
  _1 as Fa,
  x1 as Ga,
  N1 as Ha,
  R1 as Ia,
  A1 as Ja,
  O1 as Ka,
  k1 as La,
  Ao as Ma,
  sa as Na,
  od as Oa,
  ly as Pa,
  dy as Qa,
  bM as Ra,
  P1 as Sa,
  SM as Ta,
  uS as Ua,
  lS as Va,
  F1 as Wa,
  dS as Xa,
  L1 as Ya,
  j1 as Za,
  id as _a,
  V1 as $a,
  by as ab,
  My as bb,
  vS as cb,
  B1 as db,
  DS as eb,
  wS as fb,
  U1 as gb,
  $1 as hb,
  H1 as ib,
  z1 as jb,
  Ty as kb,
  sd as lb,
  _y as mb,
  CS as nb,
  q1 as ob,
  G1 as pb,
  W1 as qb,
  Z1 as rb,
  Y1 as sb,
  Q1 as tb,
  K1 as ub,
  J1 as vb,
  X1 as wb,
  eL as xb,
  tL as yb,
  nL as zb,
  rL as Ab,
  oL as Bb,
  iL as Cb,
  ua as Db,
  br as Eb,
  Po as Fb,
  GS as Gb,
  Te as Hb,
  io as Ib,
  jy as Jb,
  Zu as Kb,
  JS as Lb,
  rT as Mb,
  sL as Nb,
  Mr as Ob,
  ue as Pb,
  Tr as Qb,
  yT as Rb,
  rv as Sb,
  TL as Tb,
  _L as Ub,
  xL as Vb,
  NL as Wb,
  RL as Xb,
  AL as Yb,
  OL as Zb,
  kL as _b,
  Ed as $b,
  PL as ac,
  Cd as bc,
  FT as cc,
  QL as dc,
  mj as ec,
  y_ as fc,
  Zt as gc,
  Q_ as hc,
  Lj as ic,
  jj as jc,
  Vj as kc,
  ax as lc,
  Bj as mc,
  Wj as nc,
}; /**i18n:e2f94bf06bdfc8c8ab493a12299261c375fc525ae09e041ca331cb13279050ab*/
