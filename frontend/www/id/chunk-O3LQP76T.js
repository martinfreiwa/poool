import { a as at } from "./chunk-MO34KLTL.js";
import {
  $a as c,
  $b as N,
  Aa as Z,
  Ba as h,
  Da as Ye,
  Ea as se,
  Eb as K,
  F as A,
  Fa as H,
  Fb as ie,
  Gb as Oe,
  Ha as w,
  Hb as Ne,
  I as ue,
  Ia as ge,
  Ib as C,
  Ja as Ke,
  Jb as de,
  K as f,
  Ka as D,
  Kb as xe,
  L as _,
  La as L,
  M as q,
  Ma as a,
  Mb as Ze,
  N as $e,
  Na as s,
  Oa as g,
  P as Ge,
  Pa as Qe,
  Q as ae,
  Qa as Xe,
  Ra as le,
  Sa as F,
  Sb as j,
  T as je,
  Tb as et,
  U as u,
  Ua as ee,
  V as ze,
  Va as te,
  Wa as $,
  Wb as tt,
  X as k,
  Zb as nt,
  _ as Y,
  _a as y,
  ab as ce,
  ac as z,
  b as De,
  bb as I,
  cb as fe,
  da as re,
  db as _e,
  e as Le,
  ea as He,
  eb as he,
  fa as Ue,
  fb as be,
  fc as it,
  ga as l,
  hb as Pe,
  ib as Me,
  ic as oe,
  jb as ke,
  ka as me,
  kb as v,
  lb as ne,
  ma as Ae,
  mb as O,
  nc as ot,
  pb as We,
  qb as ve,
  ra as E,
  rb as Fe,
  ta as qe,
  ua as J,
  v as Ve,
  vb as R,
  wb as V,
  xa as x,
  xb as Je,
  z as Ee,
} from "./chunk-A4X4NSFE.js";
import { a as T, b as B, c as Re, g as Be } from "./chunk-ZBVOF6Q3.js";
var xt = (t) => ({ "translate-y-2 rotate-45": t }),
  yt = (t) => ({ "opacity-0": t }),
  Ct = (t) => ({ "-translate-y-2 -rotate-45": t }),
  wt = (t, o) => o.label;
function St(t, o) {
  if (t & 1) {
    let e = F();
    (a(0, "a", 19),
      y("click", function () {
        f(e);
        let i = c(2);
        return _(i.closeMobileMenu());
      }),
      v(1),
      s());
  }
  if (t & 2) {
    let e = o.$implicit;
    (h("routerLink", "./")("fragment", e.anchor), l(), O(" ", e.label, " "));
  }
}
function Tt(t, o) {
  if (t & 1) {
    let e = F();
    (a(0, "div", 7),
      g(1, "img", 8)(2, "div"),
      a(3, "div", 9),
      D(4, St, 2, 3, "a", 10, wt),
      a(6, "a", 11),
      y("click", function () {
        f(e);
        let i = c();
        return _(i.closeMobileMenu());
      }),
      g(7, "pool-land-button", 12),
      s(),
      g(8, "pool-land-language-selector", 13),
      s(),
      a(9, "footer", 14)(10, "p", 15),
      ee(11, 0),
      g(12, "span", 16),
      te(),
      s(),
      a(13, "h5", 17),
      $(14, 1),
      s(),
      a(15, "a", 18),
      y("click", function () {
        f(e);
        let i = c();
        return _(i.closeMobileMenu());
      }),
      $(16, 2),
      s()()());
  }
  if (t & 2) {
    let e = c();
    (l(4), L(e.navbarLinks), l(4), h("spaced", !0));
  }
}
var ei = (() => {
  class t {
    cdr = A(K);
    navbarLinks;
    isMobileMenuOpen = !1;
    ngOnInit() {
      ((this.isMobileMenuOpen = !1), this.cdr.detectChanges());
    }
    toggleMobileMenu() {
      ((this.isMobileMenuOpen = !this.isMobileMenuOpen),
        this.cdr.detectChanges());
    }
    closeMobileMenu() {
      ((this.isMobileMenuOpen = !1), this.cdr.detectChanges());
    }
    static ɵfac = function (n) {
      return new (n || t)();
    };
    static ɵcmp = E({
      type: t,
      selectors: [["pool-land-burger-nav"]],
      inputs: { navbarLinks: "navbarLinks" },
      decls: 6,
      vars: 10,
      consts: () => {
        let e;
        e = "Sign in";
        let n;
        n =
          " Thousands of investors owning shares in Bali's growing market. POOOL lets you invest in premium properties from " +
          "\uFFFD#12\uFFFD" +
          "$500" +
          "\uFFFD/#12\uFFFD" +
          ", using blockchain and notarized certificates for deals that are secure, transparent, and exclusive. ";
        let i;
        i = " Still have questions? ";
        let r;
        return (
          (r = " Contact us on telegram "),
          [
            n,
            i,
            r,
            [1, "relative", "z-50", "lg:hidden"],
            [
              "aria-label",
              "Toggle menu",
              1,
              "border-accent-green",
              "fixed",
              "right-4",
              "top-4",
              "z-50",
              "flex",
              "h-10",
              "w-10",
              "flex-col",
              "items-center",
              "justify-center",
              "gap-[6px]",
              "p-2",
              3,
              "click",
            ],
            [
              1,
              "bg-accent-green",
              "h-[2px]",
              "w-6",
              "transform",
              "transition-all",
              "duration-300",
              "ease-in-out",
              3,
              "ngClass",
            ],
            [
              1,
              "bg-accent-green",
              "h-[2px]",
              "w-6",
              "transition-all",
              "duration-300",
              "ease-in-out",
              3,
              "ngClass",
            ],
            [
              1,
              "fixed",
              "inset-0",
              "z-40",
              "flex",
              "flex-col",
              "justify-between",
              "bg-[#2B32F9]",
              "text-white",
              "transition-opacity",
              "duration-300",
              "ease-in-out",
            ],
            [
              "src",
              "/svg/logo-burger.svg",
              "alt",
              "logo",
              1,
              "fixed",
              "left-4",
              "top-[20px]",
              "z-50",
            ],
            [
              1,
              "flex",
              "flex-col",
              "items-center",
              "justify-center",
              "gap-6",
              "px-6",
              "py-12",
              "text-center",
              "text-xl",
              "font-semibold",
              "uppercase",
            ],
            [
              "queryParamsHandling",
              "preserve",
              1,
              "cursor-pointer",
              "hover:underline",
              3,
              "routerLink",
              "fragment",
            ],
            [
              "href",
              "/platform/",
              "target",
              "_blank",
              "rel",
              "noopener noreferrer",
              1,
              "mt-3",
              "w-full",
              "px-8",
              3,
              "click",
            ],
            ["text", e, "variant", "primary-full-width", 1, "w-full"],
            ["variant", "secondary", 3, "spaced"],
            [
              1,
              "flex",
              "flex-col",
              "items-center",
              "px-6",
              "pb-10",
              "text-center",
              "text-sm",
            ],
            [1, "mb-4", "text-white"],
            [1, "text-accent-green", "font-bold"],
            [1, "mb-2", "text-lg", "font-bold", "text-white"],
            [
              "href",
              "https://t.me/itspoool",
              "target",
              "_blank",
              1,
              "bg-accent-green",
              "text-primary-blue",
              "rounded-md",
              "px-5",
              "py-3",
              "font-semibold",
              3,
              "click",
            ],
            [
              "queryParamsHandling",
              "preserve",
              1,
              "cursor-pointer",
              "hover:underline",
              3,
              "click",
              "routerLink",
              "fragment",
            ],
          ]
        );
      },
      template: function (n, i) {
        (n & 1 &&
          (a(0, "nav", 3)(1, "button", 4),
          y("click", function () {
            return i.toggleMobileMenu();
          }),
          g(2, "span", 5)(3, "span", 6)(4, "span", 5),
          s(),
          x(5, Tt, 17, 1, "div", 7),
          s()),
          n & 2 &&
            (l(2),
            h("ngClass", ve(4, xt, i.isMobileMenuOpen)),
            l(),
            h("ngClass", ve(6, yt, i.isMobileMenuOpen)),
            l(),
            h("ngClass", ve(8, Ct, i.isMobileMenuOpen)),
            l(),
            w(i.isMobileMenuOpen ? 5 : -1)));
      },
      dependencies: [N, j, oe, ot, at],
      encapsulation: 2,
      changeDetection: 0,
    });
  }
  return t;
})();
var Ce = (() => {
  class t {
    platformId = A(Y);
    el = A(ze);
    ngAfterViewInit() {
      return Be(this, null, function* () {
        if (!z(this.platformId)) return;
        let e = (yield import("./chunk-HC2LBUVO.js")).gsap,
          n = (yield import("./chunk-QVPRVTS4.js")).ScrollTrigger;
        (e.registerPlugin(n),
          e.from(this.el.nativeElement, {
            scrollTrigger: {
              trigger: this.el.nativeElement,
              start: "top 85%",
              toggleActions: "play none none none",
            },
            opacity: 0,
            y: 60,
            duration: 1.2,
            ease: "expo.out",
          }));
      });
    }
    static ɵfac = function (n) {
      return new (n || t)();
    };
    static ɵdir = qe({ type: t, selectors: [["", "bubbleAnimation", ""]] });
  }
  return t;
})();
var rt = (() => {
  class t {
    sanitizer;
    constructor(e) {
      this.sanitizer = e;
    }
    transform(e) {
      return this.sanitizer.bypassSecurityTrustResourceUrl(e);
    }
    static ɵfac = function (n) {
      return new (n || t)(me(it, 16));
    };
    static ɵpipe = J({ name: "safeUrl", type: t, pure: !0 });
  }
  return t;
})();
var Et = ["video"];
function At(t, o) {
  if (t & 1) {
    let e = F();
    (a(0, "div", 43),
      y("click", function () {
        let i = f(e).$index,
          r = c();
        return _(r.activeTabIndex.set(i));
      }),
      a(1, "div", 44),
      g(2, "img", 45),
      a(3, "div", 46),
      v(4),
      s(),
      a(5, "div", 47),
      v(6),
      a(7, "span", 48),
      v(8, "POOOL"),
      s()()()());
  }
  if (t & 2) {
    let e = o.$implicit,
      n = o.$index,
      i = c();
    (Ye("border-primary-blue", i.activeTabIndex() === n),
      l(2),
      fe("alt", e.name),
      h("src", e.avatar, re),
      l(2),
      ne(e.name),
      l(2),
      O(" ", e.title, ""));
  }
}
function Pt(t, o) {
  if (t & 1) {
    let e = F();
    (a(0, "div", 49),
      y("click", function () {
        f(e);
        let i = c();
        return _(i.handleVideoClick(!1));
      }),
      a(1, "div", 50),
      y("click", function (i) {
        return (f(e), _(i.stopPropagation()));
      }),
      a(2, "button", 51),
      y("click", function () {
        f(e);
        let i = c();
        return _(i.handleVideoClick(!1));
      }),
      a(3, "span", 52),
      v(4, "\xD7"),
      s()(),
      a(5, "div", 53),
      g(6, "iframe", 54),
      R(7, "safeUrl"),
      s()()());
  }
  if (t & 2) {
    let e = c();
    (l(6), h("src", V(7, 1, e.testimonials[e.activeTabIndex()].videoSrc), He));
  }
}
var hi = (() => {
  class t {
    video;
    activeTabIndex = k(0);
    testimonials = [
      {
        name: "Monique Howeth",
        title: "Venture Investor of",
        avatar: "/webp/avatars/Monique Howeth.webp",
        videoSrc:
          "https://www.youtube.com/embed/1j-F1qATG_s?si=4_5Rh9wWAa8_WxlF?controls=1",
        thumbnailVideoSrc:
          "https://www.dropbox.com/scl/fi/zi4iug7mahm58iw0v5d87/IMG_31331.mp4?rlkey=ifl8zp0twuczqcdt29xfu0h1q&e=2&st=md9p4wi6&raw=1",
        text: "Monique Howeth is a serial entrepreneur and owner of Crunch Fitness in California. With 25 years of experience in real estate and a focus on high-growth ventures, she brings both capital and expertise to the table. In Q4 2024, she became a venture investor in POOOL, acquiring a 5% stake and backing its vision of fractional ownership as the future of investing.",
      },
      {
        name: "Jonas Thomsen",
        title: "Venture Investor of",
        avatar: "/webp/avatars/Jonas Thomsen.webp",
        videoSrc: "https://www.youtube.com/embed/RsgA3-zpMQw",
        thumbnailVideoSrc:
          "https://www.dropbox.com/scl/fi/ubh59wrjxvs9zt76rmk4i/Jonas.mov?rlkey=68wghj8hzx9f2f7uj247f0it0&st=hyn53cuj&raw=1",
        text: "Jonas Thomsen is a Danish entrepreneur and venture investor in POOOL. With experience in real estate flips across the U.S., he later expanded to Bali, focusing on short-term rentals. After recognizing the market\u2019s growth, he joined POOOL to support its mission and now helps drive its global expansion from Southeast Asia and Europe.",
      },
      {
        name: "Tobias Weber",
        title: "Venture Investor of",
        avatar: "/webp/avatars/Tobias Weber.webp",
        videoSrc: "https://www.youtube.com/embed/F4ehJx_8nCo",
        thumbnailVideoSrc:
          "https://www.dropbox.com/scl/fi/uoaasa45la2v03266ycv2/Tobias.mov?rlkey=i6kvg1gjvjs58kbvby9i27xck&st=1cuf5lfi&raw=1",
        text: "Tobias Weber is a German investor and early backer of POOOL. With a track record in real estate across the Philippines and Bali, he also invested in Bali Invest, a firm led by Jonas Freiwald. When Jonas launched POOOL, Tobias joined as a venture investor, supporting the platform\u2019s bold take on fractional ownership and real estate\u2019s future.",
      },
      {
        name: "Mathias H\xF8st Damsgaard",
        title: "Venture Investor of",
        avatar: "/webp/avatars/Mathias Damsgaard.webp",
        videoSrc:
          "https://www.youtube.com/embed/pRm0Ou4C5Ds?si=vndgnOdXvVE_OBrv",
        thumbnailVideoSrc:
          "https://www.dropbox.com/scl/fi/dlkbugzi5warmrj03rnq8/Mathias.mov?rlkey=shbgq8uu8du1vga9u1a88r3r9&st=wwnzgjby&raw=1",
        text: "Mathias H\xF8st Damsgaard is a Danish entrepreneur with experience in U.S. real estate deals and flips. After discovering Bali\u2019s strong ROI potential, he visited and met the POOOL team. Impressed by the model, he joined as a venture investor and strategic partner, now helping expand the platform across Denmark and the U.S.",
      },
    ];
    cd = A(K);
    showVideo = !1;
    handleVideoClick(e) {
      ((this.showVideo = e), this.cd.detectChanges());
    }
    ngAfterViewInit() {
      ((this.video.nativeElement.muted = !0), this.video.nativeElement.play());
    }
    static ɵfac = function (n) {
      return new (n || t)();
    };
    static ɵcmp = E({
      type: t,
      selectors: [["pool-land-buy-shares-testimonials"]],
      viewQuery: function (n, i) {
        if ((n & 1 && _e(Et, 5), n & 2)) {
          let r;
          he((r = be())) && (i.video = r.first);
        }
      },
      decls: 59,
      vars: 7,
      consts: () => {
        let e;
        e = "Testimonials";
        let n;
        n =
          " Hear from our " +
          "\uFFFD#8\uFFFD\uFFFD/#8\uFFFD" +
          " first " +
          "\uFFFD#9\uFFFD\uFFFD/#9\uFFFD" +
          "";
        let i;
        i = " investor";
        let r;
        return (
          (r = "Testimonials"),
          [
            ["video", ""],
            e,
            n,
            i,
            r,
            [
              1,
              "flex",
              "min-h-fit",
              "w-full",
              "flex-col",
              "items-center",
              "justify-center",
              "gap-4",
              "bg-[url(/svg/background-buy-shares-with-gradient-animation.svg)]",
              "bg-cover",
              "bg-center",
              "bg-no-repeat",
              "px-4",
              "py-20",
              "lg:p-20",
              "xl:p-28",
            ],
            [1, "flex", "flex-col", "items-center", "gap-6", "p-4"],
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
              "lg:text-7xl",
            ],
            [1, "block", "sm:hidden"],
            [1, "hidden", "lg:block"],
            [
              1,
              "inline-flex",
              "w-min",
              "items-center",
              "justify-start",
              "gap-2",
            ],
            [
              1,
              "-ml-[6px]",
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
              "/svg/world-map/bali.svg",
              "alt",
              "Estonia flag",
              1,
              "-mr-[8px]",
              "lg:-mr-3",
            ],
            ["src", "/svg/world-map/gb.svg", "alt", "Japan flag"],
            [1, "text-primary-blue"],
            [1, "w-full", "max-w-5xl"],
            [1, "no-scrollbar", "overflow-x-scroll", "pb-3", "lg:py-8"],
            [1, "grid", "grid-flow-col"],
            [
              1,
              "flex",
              "cursor-pointer",
              "justify-center",
              "border-0",
              "border-b-2",
              "px-4",
              "py-2",
              "text-center",
              "transition-all",
              "duration-300",
              3,
              "border-primary-blue",
            ],
            [1, "h-max", "w-full", "max-w-max"],
            [
              1,
              "flex",
              "flex-col",
              "items-center",
              "justify-center",
              "gap-3",
              "lg:h-[412px]",
              "lg:flex-row",
              "lg:gap-4",
            ],
            [
              1,
              "relative",
              "flex",
              "aspect-square",
              "h-full",
              "w-full",
              "overflow-hidden",
              "rounded-xl",
            ],
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
              1,
              "object-cover",
              3,
              "src",
            ],
            [
              "tabindex",
              "0",
              "role",
              "button",
              "aria-label",
              "Open video",
              1,
              "absolute",
              "inset-0",
              "flex",
              "cursor-pointer",
              "items-center",
              "justify-center",
              "bg-black",
              "bg-opacity-40",
              3,
              "click",
              "keydown.enter",
              "keydown.space",
            ],
            [
              "aria-label",
              "Play video",
              1,
              "bg-primary-blue",
              "flex",
              "h-16",
              "w-16",
              "items-center",
              "justify-center",
              "rounded-full",
              "bg-opacity-80",
              "text-white",
              "transition-transform",
              "hover:scale-110",
              3,
              "keydown.enter",
            ],
            [
              "xmlns",
              "http://www.w3.org/2000/svg",
              "viewBox",
              "0 0 24 24",
              "fill",
              "currentColor",
              1,
              "h-8",
              "w-8",
            ],
            ["d", "M8 5v14l11-7z"],
            [
              1,
              "h-full",
              "max-w-[540px]",
              "rounded-xl",
              "border",
              "bg-white",
              "p-4",
              "text-left",
              "lg:px-8",
            ],
            [1, "flex", "justify-end", "p-2"],
            [1, "flex", "items-baseline", "gap-2"],
            ["src", "/svg/logo-blue.svg", "alt", "pool-logo", 1, "h-5"],
            [1, "text-lg"],
            [
              1,
              "pb-8",
              "pt-4",
              "font-medium",
              "text-gray-800",
              "lg:pb-8",
              "lg:pr-8",
            ],
            [1, "text-primary-blue", "text-5xl"],
            [
              1,
              "grid",
              "grid-cols-[1fr_max-content]",
              "items-center",
              "justify-center",
              "gap-4",
            ],
            [1, "div"],
            [1, "font-medium"],
            [1, "text-muted", "text-xs"],
            [1, "text-primary-blue", "font-medium", "-tracking-[0.15em]"],
            [1, "text-primary-blue", "self-center", "text-5xl"],
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
            ],
            [
              1,
              "flex",
              "cursor-pointer",
              "justify-center",
              "border-0",
              "border-b-2",
              "px-4",
              "py-2",
              "text-center",
              "transition-all",
              "duration-300",
              3,
              "click",
            ],
            [
              1,
              "grid",
              "w-fit",
              "grid-cols-[max-content_max-content]",
              "grid-rows-2",
              "items-center",
              "justify-items-start",
              "gap-x-4",
            ],
            [
              1,
              "row-span-3",
              "mx-auto",
              "h-12",
              "w-12",
              "rounded-full",
              "object-cover",
              3,
              "src",
              "alt",
            ],
            [1, "text-sm", "font-medium"],
            [1, "min-w-max", "text-xs", "text-gray-500"],
            [
              1,
              "text-primary-blue",
              "ml-1",
              "text-xs",
              "font-medium",
              "-tracking-[0.15em]",
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
      template: function (n, i) {
        if (n & 1) {
          let r = F();
          (a(0, "div", 5)(1, "div", 6)(2, "span", 7),
            $(3, 1),
            s(),
            a(4, "div", 8)(5, "h2"),
            Qe(6),
            ee(7, 2),
            g(8, "br", 9)(9, "br", 10),
            te(),
            Xe(),
            a(10, "div", 11),
            v(11, " Gl "),
            a(12, "div", 12),
            g(13, "img", 13)(14, "img", 14)(15, "img", 15),
            s(),
            v(16, " bal "),
            s(),
            g(17, "br", 9),
            a(18, "span", 16),
            $(19, 3),
            s()()()(),
            a(20, "div", 17)(21, "div", 18)(22, "div", 19),
            D(23, At, 9, 6, "div", 20, ge),
            s()(),
            a(25, "div", 21)(26, "div", 21)(27, "div", 21)(28, "div", 22)(
              29,
              "div",
              23,
            ),
            g(30, "video", 24, 0),
            R(32, "safeUrl"),
            a(33, "div", 25),
            y("click", function () {
              return (f(r), _(i.handleVideoClick(!0)));
            })("keydown.enter", function () {
              return (f(r), _(i.handleVideoClick(!0)));
            })("keydown.space", function (b) {
              return (f(r), i.handleVideoClick(!0), _(b.preventDefault()));
            }),
            a(34, "button", 26),
            y("keydown.enter", function () {
              return (f(r), _(i.handleVideoClick(!0)));
            }),
            q(),
            a(35, "svg", 27),
            g(36, "path", 28),
            s()()()(),
            $e(),
            a(37, "div", 29)(38, "div", 30)(39, "span", 31),
            g(40, "img", 32),
            a(41, "span", 33),
            $(42, 4),
            s()()(),
            a(43, "p", 34)(44, "span", 35),
            v(45, "\u275D"),
            s(),
            g(46, "br"),
            v(47),
            s(),
            a(48, "div", 36)(49, "div", 37)(50, "div", 38),
            v(51),
            s(),
            a(52, "div", 39),
            v(53),
            a(54, "span", 40),
            v(55, " POOOL "),
            s()()(),
            a(56, "span", 41),
            v(57, "\u275E"),
            s()()()()()()(),
            x(58, Pt, 8, 3, "div", 42),
            s()());
        }
        n & 2 &&
          (l(23),
          L(i.testimonials),
          l(7),
          h(
            "src",
            V(32, 5, i.testimonials[i.activeTabIndex()].thumbnailVideoSrc),
            re,
          ),
          l(17),
          O(' "', i.testimonials[i.activeTabIndex()].text, '" '),
          l(4),
          O(" ", i.testimonials[i.activeTabIndex()].name, " "),
          l(2),
          O(" ", i.testimonials[i.activeTabIndex()].title, " "),
          l(5),
          w(i.showVideo ? 58 : -1));
      },
      dependencies: [N, Ce, rt],
      encapsulation: 2,
      changeDetection: 0,
    });
  }
  return t;
})();
var Mt = (t, o) => ({ transform: t, opacity: o }),
  kt = (t, o) => ({ "grid-rows-[1fr]": t, "grid-rows-[0fr]": o }),
  Ft = (t, o) => o.question;
function Ot(t, o) {
  t & 1 && g(0, "hr", 31);
}
function Nt(t, o) {
  if (t & 1) {
    let e = F();
    (a(0, "div", 22)(1, "div", 23),
      y("click", function () {
        let i = f(e).$implicit;
        return _((i.expanded = !i.expanded));
      }),
      a(2, "span", 24),
      v(3),
      s(),
      a(4, "div", 25),
      g(5, "div", 26)(6, "div", 27),
      s()(),
      a(7, "div", 28)(8, "div", 29)(9, "p", 30),
      v(10),
      s()()()(),
      x(11, Ot, 1, 0, "hr", 31));
  }
  if (t & 2) {
    let e = o.$implicit,
      n = c();
    (l(3),
      ne(e.question),
      l(3),
      h(
        "ngStyle",
        Fe(
          5,
          Mt,
          e.expanded ? "scaleY(0)" : "scaleY(1)",
          e.expanded ? "0" : "1",
        ),
      ),
      l(),
      h("ngClass", Fe(8, kt, e.expanded, !e.expanded)),
      l(3),
      O(" ", e.answer, " "),
      l(),
      w(e.question !== n.faqs()[n.faqs().length - 1].question ? 11 : -1));
  }
}
var Si = (() => {
  class t {
    cd = A(K);
    faqs = u.required();
    activeTabIndex = 0;
    tabs = ["General", "Legal"];
    handleTabClick(e) {
      ((this.activeTabIndex = e), this.cd.detectChanges());
    }
    static ɵfac = function (n) {
      return new (n || t)();
    };
    static ɵcmp = E({
      type: t,
      selectors: [["pool-land-faq"]],
      inputs: { faqs: [1, "faqs"] },
      decls: 27,
      vars: 0,
      consts: () => {
        let e;
        e =
          " Frequently" +
          "\uFFFD#4\uFFFD\uFFFD/#4\uFFFD" +
          " asked " +
          "\uFFFD#5\uFFFD" +
          "questions" +
          "\uFFFD/#5\uFFFD" +
          "";
        let n;
        n = " Everything you need to know about the product ";
        let i;
        i = "Still have questions?";
        let r;
        return (
          (r =
            "Can't find the answer you're looking for? Please chat to our friendly team."),
          [
            e,
            n,
            i,
            r,
            [
              1,
              "max-block-width",
              "flex",
              "w-screen",
              "flex-col",
              "items-center",
              "justify-center",
              "gap-6",
              "px-4",
              "py-20",
              "lg:gap-8",
              "lg:px-20",
              "lg:py-28",
              "xl:p-28",
            ],
            [
              1,
              "flex",
              "flex-col",
              "items-center",
              "justify-center",
              "gap-4",
              "lg:gap-5",
            ],
            [
              "bubbleAnimation",
              "",
              1,
              "text-center",
              "text-5xl",
              "font-extrabold",
              "uppercase",
              "tracking-tighter",
            ],
            [1, "text-primary-blue"],
            [1, "text-muted", "text-center"],
            [1, "flex", "w-[300px]", "flex-col", "gap-8", "lg:w-[700px]"],
            [1, "flex", "flex-col", "items-center", "justify-center", "gap-8"],
            [
              1,
              "flex",
              "flex-col",
              "rounded-xl",
              "border",
              "border-gray-200",
              "bg-white",
              "py-4",
            ],
            [
              "id",
              "contactUs",
              1,
              "flex",
              "flex-col",
              "items-center",
              "justify-center",
              "gap-8",
              "rounded-xl",
              "border",
              "border-gray-200",
              "bg-white",
              "p-2",
              "py-8",
              "lg:bg-[#03FF88]",
              "lg:py-10",
            ],
            ["src", "png/Team.webp", "alt", "FAQ service team", 1, "w-32"],
            [1, "flex", "h-full", "flex-col", "text-center"],
            [1, "text-xl", "font-bold", "lg:text-2xl"],
            [1, "max-w-[480px]", "px-4", "py-4", "text-gray-600", "lg:text-lg"],
            [1, "flex", "gap-2", "lg:gap-6"],
            ["href", "https://t.me/dmitry_sikorski", "target", "_blank"],
            [
              1,
              "bg-primary-blue",
              "hover:text-primary-blue",
              "hover:bg-secondary-green",
              "rounded-full",
              "px-6",
              "py-4",
              "font-semibold",
              "text-white",
              "transition-all",
              "duration-300",
              "lg:text-lg",
              "lg:hover:bg-white",
            ],
            ["href", "https://wa.me/6287820057942", "target", "_blank"],
            [
              1,
              "bg-secondary-green",
              "hover:bg-primary-blue",
              "text-primary-blue",
              "lg:hover:bg-primary-blue",
              "rounded-full",
              "px-6",
              "py-4",
              "font-semibold",
              "transition-all",
              "duration-300",
              "hover:text-white",
              "lg:bg-white",
              "lg:text-lg",
            ],
            [1, "group", "p-12", "py-6", "lg:px-10"],
            [
              1,
              "flex",
              "grow",
              "cursor-pointer",
              "list-none",
              "items-center",
              "justify-between",
              "gap-4",
              3,
              "click",
            ],
            [1, "text-xl", "font-medium", "lg:text-2xl"],
            [
              1,
              "border-primary-blue",
              "relative",
              "flex",
              "h-6",
              "w-6",
              "flex-shrink-0",
              "items-center",
              "justify-center",
              "rounded-full",
              "border-2",
            ],
            [
              1,
              "bg-primary-blue",
              "absolute",
              "h-[2px]",
              "w-3",
              "rounded-full",
            ],
            [
              1,
              "bg-primary-blue",
              "absolute",
              "h-3",
              "w-[2px]",
              "origin-center",
              "rounded-full",
              "transition-all",
              "duration-300",
              "ease-out",
              3,
              "ngStyle",
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
            [1, "text-muted", "whitespace-pre-line", "py-2", "text-sm"],
            [1, "w-full", "border-t", "border-gray-200"],
          ]
        );
      },
      template: function (n, i) {
        (n & 1 &&
          (a(0, "div", 4)(1, "div", 5)(2, "h3", 6),
          ee(3, 0),
          g(4, "br")(5, "span", 7),
          te(),
          s(),
          a(6, "h4", 8),
          $(7, 1),
          s()(),
          a(8, "div", 9)(9, "div", 10)(10, "div", 11),
          D(11, Nt, 12, 11, null, null, Ft),
          s()(),
          a(13, "div", 12),
          g(14, "img", 13),
          a(15, "p", 14)(16, "span", 15),
          $(17, 2),
          s(),
          a(18, "span", 16),
          $(19, 3),
          s()(),
          a(20, "div", 17)(21, "a", 18)(22, "button", 19),
          v(23, " Telegram "),
          s()(),
          a(24, "a", 20)(25, "button", 21),
          v(26, " WhatsApp "),
          s()()()()()()),
          n & 2 && (l(11), L(i.faqs())));
      },
      dependencies: [N, j, tt, Ce],
      styles: [
        "[_nghost-%COMP%]{display:flex;height:100%;width:100%;--tw-bg-opacity: 1;background-color:rgb(250 250 250 / var(--tw-bg-opacity, 1))}",
      ],
      changeDetection: 0,
    });
  }
  return t;
})();
function It(t, o) {
  t & 1 && (q(), a(0, "svg", 0), g(1, "path", 2), s());
}
function Rt(t, o) {
  t & 1 && (q(), a(0, "svg", 0), g(1, "path", 3), s());
}
function Bt(t, o) {
  t & 1 && (q(), a(0, "svg", 0), g(1, "path", 4), s());
}
function Dt(t, o) {
  t & 1 && (q(), a(0, "svg", 1), g(1, "path", 5)(2, "path", 6), s());
}
function Lt(t, o) {
  t & 1 && g(0, "div", 2);
}
var Vt = ["toastRef"],
  ct = [
    [["", "loading-icon", ""]],
    [["", "success-icon", ""]],
    [["", "error-icon", ""]],
    [["", "warning-icon", ""]],
    [["", "info-icon", ""]],
  ],
  dt = [
    "[loading-icon]",
    "[success-icon]",
    "[error-icon]",
    "[warning-icon]",
    "[info-icon]",
  ];
function $t(t, o) {
  if (t & 1) {
    let e = F();
    (a(0, "button", 3),
      y("click", function () {
        f(e);
        let i = c();
        return _(i.onCloseButtonClick());
      }),
      q(),
      a(1, "svg", 4),
      g(2, "line", 5)(3, "line", 6),
      s()());
  }
  if (t & 2) {
    let e,
      n = c();
    (H(
      n.cn(
        n.classes().closeButton,
        (e = n.toast().classes) == null ? null : e.closeButton,
      ),
    ),
      Z("data-disabled", n.disabled()));
  }
}
function Gt(t, o) {
  t & 1 && le(0);
}
function jt(t, o) {
  if (
    (t & 1 && (x(0, Gt, 1, 0, "ng-container", 7), R(1, "asComponent")), t & 2)
  ) {
    let e = c();
    h("ngComponentOutlet", V(1, 2, e.toast().component))(
      "ngComponentOutletInputs",
      e.toast().componentProps,
    );
  }
}
function zt(t, o) {
  t & 1 && I(0);
}
function Ht(t, o) {
  t & 1 && le(0);
}
function Ut(t, o) {
  if (
    (t & 1 && (x(0, Ht, 1, 0, "ng-container", 7), R(1, "asComponent")), t & 2)
  ) {
    let e = c(3);
    h("ngComponentOutlet", V(1, 2, e.toast().icon))(
      "ngComponentOutletInputs",
      e.toast().componentProps,
    );
  }
}
function qt(t, o) {
  t & 1 && I(0, 1);
}
function Yt(t, o) {
  t & 1 && I(0, 2);
}
function Kt(t, o) {
  t & 1 && I(0, 3);
}
function Qt(t, o) {
  t & 1 && I(0, 4);
}
function Xt(t, o) {
  if ((t & 1 && x(0, qt, 1, 0)(1, Yt, 1, 0)(2, Kt, 1, 0)(3, Qt, 1, 0), t & 2)) {
    let e,
      n = c(3);
    w(
      (e = n.toastType()) === "success"
        ? 0
        : e === "error"
          ? 1
          : e === "warning"
            ? 2
            : e === "info"
              ? 3
              : -1,
    );
  }
}
function Wt(t, o) {
  if (
    (t & 1 &&
      (a(0, "div", 8),
      x(1, zt, 1, 0)(2, Ut, 2, 4, "ng-container")(3, Xt, 4, 1),
      s()),
    t & 2)
  ) {
    let e = c(2);
    (l(),
      w(e.toastType() === "loading" && !e.toast().icon ? 1 : -1),
      l(),
      w(e.toast().icon ? 2 : 3));
  }
}
function Jt(t, o) {
  if ((t & 1 && v(0), t & 2)) {
    let e = c(3);
    O(" ", e.toast().title, " ");
  }
}
function Zt(t, o) {
  t & 1 && le(0);
}
function en(t, o) {
  if (
    (t & 1 && (x(0, Zt, 1, 0, "ng-container", 7), R(1, "asComponent")), t & 2)
  ) {
    let e = c(),
      n = c(2);
    h("ngComponentOutlet", V(1, 2, e))(
      "ngComponentOutletInputs",
      n.toast().componentProps,
    );
  }
}
function tn(t, o) {
  if (
    (t & 1 &&
      (a(0, "div", 14),
      x(1, Jt, 1, 1),
      R(2, "isString"),
      x(3, en, 2, 4, "ng-container"),
      s()),
    t & 2)
  ) {
    let e,
      n = c(2);
    (H(
      n.cn(n.classes().title, (e = n.toast().classes) == null ? null : e.title),
    ),
      l(),
      w(V(2, 3, o) ? 1 : 3));
  }
}
function nn(t, o) {
  if ((t & 1 && v(0), t & 2)) {
    let e = c(3);
    O(" ", e.toast().description, " ");
  }
}
function on(t, o) {
  t & 1 && le(0);
}
function an(t, o) {
  if (
    (t & 1 && (x(0, on, 1, 0, "ng-container", 7), R(1, "asComponent")), t & 2)
  ) {
    let e = c(),
      n = c(2);
    h("ngComponentOutlet", V(1, 2, e))(
      "ngComponentOutletInputs",
      n.toast().componentProps,
    );
  }
}
function rn(t, o) {
  if (
    (t & 1 &&
      (a(0, "div", 15),
      x(1, nn, 1, 1),
      R(2, "isString"),
      x(3, an, 2, 4, "ng-container"),
      s()),
    t & 2)
  ) {
    let e,
      n = c(2);
    (H(
      n.cn(
        n.descriptionClass(),
        n.toastDescriptionClass(),
        n.classes().description,
        (e = n.toast().classes) == null ? null : e.description,
      ),
    ),
      l(),
      w(V(2, 3, o) ? 1 : 3));
  }
}
function sn(t, o) {
  if (t & 1) {
    let e = F();
    (a(0, "button", 16),
      y("click", function () {
        f(e);
        let i = c(2);
        return _(i.onCancelClick());
      }),
      v(1),
      s());
  }
  if (t & 2) {
    let e,
      n,
      i = c(2);
    (se(
      (e = i.cancelButtonStyle()) !== null && e !== void 0
        ? e
        : i.toast().cancelButtonStyle,
    ),
      H(
        i.cn(
          i.classes().cancelButton,
          (n = i.toast().classes) == null ? null : n.cancelButton,
        ),
      ),
      l(),
      O(" ", o.label, " "));
  }
}
function ln(t, o) {
  if (t & 1) {
    let e = F();
    (a(0, "button", 17),
      y("click", function (i) {
        f(e);
        let r = c(2);
        return _(r.onActionClick(i));
      }),
      v(1),
      s());
  }
  if (t & 2) {
    let e,
      n,
      i = c(2);
    (se(
      (e = i.actionButtonStyle()) !== null && e !== void 0
        ? e
        : i.toast().actionButtonStyle,
    ),
      H(
        i.cn(
          i.classes().actionButton,
          (n = i.toast().classes) == null ? null : n.actionButton,
        ),
      ),
      l(),
      O(" ", o.label, " "));
  }
}
function cn(t, o) {
  if (
    (t & 1 &&
      (x(0, Wt, 4, 2, "div", 8),
      a(1, "div", 9),
      x(2, tn, 4, 5, "div", 10)(3, rn, 4, 5, "div", 11),
      s(),
      x(4, sn, 2, 5, "button", 12)(5, ln, 2, 5, "button", 13)),
    t & 2)
  ) {
    let e,
      n,
      i,
      r,
      d = c();
    (w(
      d.toastType() !== "default" || d.toast().icon || d.toast().promise
        ? 0
        : -1,
    ),
      l(2),
      w((e = d.toast().title) ? 2 : -1, e),
      l(),
      w((n = d.toast().description) ? 3 : -1, n),
      l(),
      w((i = d.toast().cancel) ? 4 : -1, i),
      l(),
      w((r = d.toast().action) ? 5 : -1, r));
  }
}
var dn = ["listRef"],
  pn = () => ({}),
  un = (t, o) => o.id;
function mn(t, o) {
  if ((t & 1 && g(0, "ngx-sonner-loader", 6), t & 2)) {
    let e = c().$implicit;
    h("isVisible", e.type === "loading");
  }
}
function gn(t, o) {
  t & 1 && g(0, "ngx-sonner-icon", 7);
}
function fn(t, o) {
  t & 1 && g(0, "ngx-sonner-icon", 8);
}
function _n(t, o) {
  t & 1 && g(0, "ngx-sonner-icon", 9);
}
function hn(t, o) {
  t & 1 && g(0, "ngx-sonner-icon", 10);
}
function bn(t, o) {
  if (
    (t & 1 &&
      (a(0, "ngx-sonner-toast", 5),
      I(1, 0, ["loading-icon", ""], mn, 1, 1),
      I(3, 1, ["success-icon", ""], gn, 1, 0),
      I(5, 2, ["error-icon", ""], fn, 1, 0),
      I(7, 3, ["warning-icon", ""], _n, 1, 0),
      I(9, 4, ["info-icon", ""], hn, 1, 0),
      s()),
    t & 2)
  ) {
    let e,
      n,
      i,
      r,
      d,
      b = o.$implicit,
      pe = o.$index,
      M = c(3);
    (H((e = M.toastOptions().class) !== null && e !== void 0 ? e : ""),
      h("index", pe)("toast", b)("invert", M.invert())(
        "visibleToasts",
        M.visibleToasts(),
      )("closeButton", M.closeButton())("interacting", M.interacting())(
        "position",
        M.position(),
      )("expandByDefault", M.expand())("expanded", M.expanded())(
        "actionButtonStyle",
        M.toastOptions().actionButtonStyle,
      )("cancelButtonStyle", M.toastOptions().cancelButtonStyle)(
        "descriptionClass",
        (n = M.toastOptions().descriptionClass) !== null && n !== void 0
          ? n
          : "",
      )(
        "classes",
        (i = M.toastOptions().classes) !== null && i !== void 0
          ? i
          : We(17, pn),
      )(
        "duration",
        (r = M.toastOptions().duration) !== null && r !== void 0
          ? r
          : M.duration(),
      )(
        "unstyled",
        (d = M.toastOptions().unstyled) !== null && d !== void 0 ? d : !1,
      ));
  }
}
function vn(t, o) {
  if (t & 1) {
    let e = F();
    (a(0, "ol", 3, 0),
      y("blur", function (i) {
        f(e);
        let r = c(2);
        return _(r.handleBlur(i));
      })("focus", function (i) {
        f(e);
        let r = c(2);
        return _(r.handleFocus(i));
      })("mouseenter", function () {
        f(e);
        let i = c(2);
        return _(i.expanded.set(!0));
      })("mousemove", function () {
        f(e);
        let i = c(2);
        return _(i.expanded.set(!0));
      })("mouseleave", function () {
        f(e);
        let i = c(2);
        return _(i.handleMouseLeave());
      })("pointerdown", function (i) {
        f(e);
        let r = c(2);
        return _(r.handlePointerDown(i));
      })("pointerup", function () {
        f(e);
        let i = c(2);
        return _(i.interacting.set(!1));
      }),
      D(2, bn, 11, 18, "ngx-sonner-toast", 4, un),
      R(4, "toastFilter"),
      s());
  }
  if (t & 2) {
    let e = o.$implicit,
      n = o.$index,
      i = c(2);
    (se(i.toasterStyles()),
      H(i._class()),
      h("tabIndex", -1),
      Z("data-theme", i.actualTheme())("data-rich-colors", i.richColors())(
        "dir",
        i.dir() === "auto" ? i.getDocumentDirection() : i.dir(),
      )("data-y-position", e.split("-")[0])("data-x-position", e.split("-")[1]),
      l(2),
      L(Je(4, 10, i.toasts(), n, e)));
  }
}
function xn(t, o) {
  if (
    (t & 1 && (a(0, "section", 1), D(1, vn, 5, 14, "ol", 2, Ke), s()), t & 2)
  ) {
    let e = c();
    (h("tabIndex", -1),
      Z("aria-label", "Notifications " + e.hotKeyLabel()),
      l(),
      L(e.possiblePositions()));
  }
}
var st = 0;
function yn() {
  let t = k([]),
    o = k([]);
  function e(m) {
    t.update((p) => [m, ...p]);
  }
  function n(m) {
    let Ie = m,
      { message: p } = Ie,
      S = Re(Ie, ["message"]),
      Q = typeof m?.id == "number" || (m.id && m.id?.length > 0) ? m.id : st++,
      X = m.dismissible ?? !0,
      G = m.type ?? "default";
    return (
      t().find((Se) => Se.id === Q)
        ? t.update((Se) =>
            Se.map((Te) =>
              Te.id === Q
                ? B(T(T({}, Te), m), {
                    id: Q,
                    title: p,
                    dismissible: X,
                    type: G,
                    updated: !0,
                  })
                : B(T({}, Te), { updated: !1 }),
            ),
          )
        : e(B(T({}, S), { id: Q, title: p, dismissible: X, type: G })),
      Q
    );
  }
  function i(m) {
    if (m === void 0) {
      t.set([]);
      return;
    }
    return (t.update((p) => p.filter((S) => S.id !== m)), m);
  }
  function r(m, p) {
    return n(B(T({}, p), { type: "default", message: m }));
  }
  function d(m, p) {
    return n(B(T({}, p), { type: "error", message: m }));
  }
  function b(m, p) {
    return n(B(T({}, p), { type: "success", message: m }));
  }
  function pe(m, p) {
    return n(B(T({}, p), { type: "info", message: m }));
  }
  function M(m, p) {
    return n(B(T({}, p), { type: "warning", message: m }));
  }
  function mt(m, p) {
    return n(B(T({}, p), { type: "loading", message: m }));
  }
  function gt(m, p) {
    if (!p) return;
    let S;
    p.loading !== void 0 &&
      (S = n(B(T({}, p), { promise: m, type: "loading", message: p.loading })));
    let Q = m instanceof Promise ? m : m(),
      X = S !== void 0;
    return (
      Q.then((G) => {
        if (G && typeof G.ok == "boolean" && !G.ok) {
          X = !1;
          let W =
            typeof p.error == "function"
              ? p.error(`HTTP error! status: ${G.status}`)
              : p.error;
          n({ id: S, type: "error", message: W });
        } else if (p.success !== void 0) {
          X = !1;
          let W = typeof p.success == "function" ? p.success(G) : p.success;
          n({ id: S, type: "success", message: W });
        }
      })
        .catch((G) => {
          if (p.error !== void 0) {
            X = !1;
            let W = typeof p.error == "function" ? p.error(G) : p.error;
            n({ id: S, type: "error", message: W });
          }
        })
        .finally(() => {
          (X && (i(S), (S = void 0)), p.finally?.());
        }),
      S
    );
  }
  function ft(m, p) {
    let S = p?.id ?? st++;
    return (n(T({ component: m, id: S }, p)), S);
  }
  function _t(m) {
    o.update((p) => p.filter((S) => S.toastId !== m));
  }
  function ht(m) {
    o.update((p) => [m, ...p].sort(bt));
  }
  let bt = (m, p) =>
    t().findIndex((S) => S.id === m.toastId) -
    t().findIndex((S) => S.id === p.toastId);
  function vt() {
    (t.set([]), o.set([]));
  }
  return {
    create: n,
    addToast: e,
    dismiss: i,
    message: r,
    error: d,
    success: b,
    info: pe,
    warning: M,
    loading: mt,
    promise: gt,
    custom: ft,
    removeHeight: _t,
    addHeight: ht,
    reset: vt,
    toasts: t.asReadonly(),
    heights: o.asReadonly(),
  };
}
var P = yn();
function Cn(t, o) {
  return P.create(T({ message: t }, o));
}
var wn = Cn,
  Oi = Object.assign(wn, {
    success: P.success,
    info: P.info,
    warning: P.warning,
    error: P.error,
    custom: P.custom,
    message: P.message,
    promise: P.promise,
    dismiss: P.dismiss,
    loading: P.loading,
  }),
  Sn = (() => {
    class t {
      constructor() {
        this.type = u("default");
      }
      static {
        this.ɵfac = function (n) {
          return new (n || t)();
        };
      }
      static {
        this.ɵcmp = E({
          type: t,
          selectors: [["ngx-sonner-icon"]],
          inputs: { type: [1, "type"] },
          decls: 4,
          vars: 1,
          consts: [
            [
              "xmlns",
              "http://www.w3.org/2000/svg",
              "viewBox",
              "0 0 20 20",
              "fill",
              "currentColor",
              "height",
              "20",
              "width",
              "20",
            ],
            [
              "viewBox",
              "0 0 64 64",
              "fill",
              "currentColor",
              "height",
              "20",
              "width",
              "20",
              "xmlns",
              "http://www.w3.org/2000/svg",
            ],
            [
              "fill-rule",
              "evenodd",
              "d",
              "M10 18a8 8 0 100-16 8 8 0 000 16zm3.857-9.809a.75.75 0 00-1.214-.882l-3.483 4.79-1.88-1.88a.75.75 0 10-1.06 1.061l2.5 2.5a.75.75 0 001.137-.089l4-5.5z",
              "clip-rule",
              "evenodd",
            ],
            [
              "fill-rule",
              "evenodd",
              "d",
              "M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-8-5a.75.75 0 01.75.75v4.5a.75.75 0 01-1.5 0v-4.5A.75.75 0 0110 5zm0 10a1 1 0 100-2 1 1 0 000 2z",
              "clip-rule",
              "evenodd",
            ],
            [
              "fill-rule",
              "evenodd",
              "d",
              "M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a.75.75 0 000 1.5h.253a.25.25 0 01.244.304l-.459 2.066A1.75 1.75 0 0010.747 15H11a.75.75 0 000-1.5h-.253a.25.25 0 01-.244-.304l.459-2.066A1.75 1.75 0 009.253 9H9z",
              "clip-rule",
              "evenodd",
            ],
            [
              "d",
              "M32.427,7.987c2.183,0.124 4,1.165 5.096,3.281l17.936,36.208c1.739,3.66 -0.954,8.585 -5.373,8.656l-36.119,0c-4.022,-0.064 -7.322,-4.631 -5.352,-8.696l18.271,-36.207c0.342,-0.65 0.498,-0.838 0.793,-1.179c1.186,-1.375 2.483,-2.111 4.748,-2.063Zm-0.295,3.997c-0.687,0.034 -1.316,0.419 -1.659,1.017c-6.312,11.979 -12.397,24.081 -18.301,36.267c-0.546,1.225 0.391,2.797 1.762,2.863c12.06,0.195 24.125,0.195 36.185,0c1.325,-0.064 2.321,-1.584 1.769,-2.85c-5.793,-12.184 -11.765,-24.286 -17.966,-36.267c-0.366,-0.651 -0.903,-1.042 -1.79,-1.03Z",
            ],
            [
              "d",
              "M33.631,40.581l-3.348,0l-0.368,-16.449l4.1,0l-0.384,16.449Zm-3.828,5.03c0,-0.609 0.197,-1.113 0.592,-1.514c0.396,-0.4 0.935,-0.601 1.618,-0.601c0.684,0 1.223,0.201 1.618,0.601c0.395,0.401 0.593,0.905 0.593,1.514c0,0.587 -0.193,1.078 -0.577,1.473c-0.385,0.395 -0.929,0.593 -1.634,0.593c-0.705,0 -1.249,-0.198 -1.634,-0.593c-0.384,-0.395 -0.576,-0.886 -0.576,-1.473Z",
            ],
          ],
          template: function (n, i) {
            if (
              (n & 1 &&
                x(0, It, 2, 0, ":svg:svg", 0)(1, Rt, 2, 0, ":svg:svg", 0)(
                  2,
                  Bt,
                  2,
                  0,
                  ":svg:svg",
                  0,
                )(3, Dt, 3, 0, ":svg:svg", 1),
              n & 2)
            ) {
              let r;
              w(
                (r = i.type()) === "success"
                  ? 0
                  : r === "error"
                    ? 1
                    : r === "info"
                      ? 2
                      : r === "warning"
                        ? 3
                        : -1,
              );
            }
          },
          encapsulation: 2,
          changeDetection: 0,
        });
      }
    }
    return t;
  })(),
  Tn = 3,
  En = "32px",
  we = 4e3,
  An = 356,
  pt = 14,
  Pn = 20,
  Mn = 200,
  kn = {
    toast: "",
    title: "",
    description: "",
    loader: "",
    closeButton: "",
    cancelButton: "",
    actionButton: "",
    action: "",
    warning: "",
    error: "",
    success: "",
    default: "",
    info: "",
    loading: "",
  },
  Fn = (() => {
    class t {
      constructor() {
        ((this.isVisible = u.required({ transform: ie })),
          (this.bars = Array(12).fill(0)));
      }
      static {
        this.ɵfac = function (n) {
          return new (n || t)();
        };
      }
      static {
        this.ɵcmp = E({
          type: t,
          selectors: [["ngx-sonner-loader"]],
          inputs: { isVisible: [1, "isVisible"] },
          decls: 4,
          vars: 1,
          consts: [
            [1, "sonner-loading-wrapper"],
            [1, "sonner-spinner"],
            [1, "sonner-loading-bar"],
          ],
          template: function (n, i) {
            (n & 1 &&
              (a(0, "div", 0)(1, "div", 1),
              D(2, Lt, 1, 0, "div", 2, ge),
              s()()),
              n & 2 && (Z("data-visible", i.isVisible()), l(2), L(i.bars)));
          },
          encapsulation: 2,
          changeDetection: 0,
        });
      }
    }
    return t;
  })(),
  On = (() => {
    class t {
      transform(e, n, i) {
        return e.filter((r) => (!r.position && n === 0) || r.position === i);
      }
      static {
        this.ɵfac = function (n) {
          return new (n || t)();
        };
      }
      static {
        this.ɵpipe = J({ name: "toastFilter", type: t, pure: !0 });
      }
    }
    return t;
  })();
function lt(...t) {
  return t.filter(Boolean).join(" ");
}
var Nn = (() => {
    class t {
      transform(e) {
        return e;
      }
      static {
        this.ɵfac = function (n) {
          return new (n || t)();
        };
      }
      static {
        this.ɵpipe = J({ name: "asComponent", type: t, pure: !0 });
      }
    }
    return t;
  })(),
  In = (() => {
    class t {
      transform(e) {
        return typeof e == "string";
      }
      static {
        this.ɵfac = function (n) {
          return new (n || t)();
        };
      }
      static {
        this.ɵpipe = J({ name: "isString", type: t, pure: !0 });
      }
    }
    return t;
  })(),
  Rn = (() => {
    class t {
      constructor() {
        ((this.cn = lt),
          (this.toasts = P.toasts),
          (this.heights = P.heights),
          (this.removeHeight = P.removeHeight),
          (this.addHeight = P.addHeight),
          (this.dismiss = P.dismiss),
          (this.toast = u.required()),
          (this.index = u.required()),
          (this.expanded = u.required()),
          (this._invert = u.required({ alias: "invert" })),
          (this.position = u.required()),
          (this.visibleToasts = u.required()),
          (this.expandByDefault = u.required()),
          (this._closeButton = u.required({ alias: "closeButton" })),
          (this.interacting = u.required()),
          (this.cancelButtonStyle = u()),
          (this.actionButtonStyle = u()),
          (this.duration = u(we)),
          (this.descriptionClass = u("")),
          (this._classes = u({}, { alias: "classes" })),
          (this.unstyled = u(!1)),
          (this._class = u("", { alias: "class" })),
          (this._style = u({}, { alias: "style" })),
          (this.mounted = k(!1)),
          (this.removed = k(!1)),
          (this.swiping = k(!1)),
          (this.swipeOut = k(!1)),
          (this.offsetBeforeRemove = k(0)),
          (this.initialHeight = k(0)),
          (this.toastRef = Ae.required("toastRef")),
          (this.classes = C(() => T(T({}, kn), this._classes()))),
          (this.isFront = C(() => this.index() === 0)),
          (this.isVisible = C(() => this.index() + 1 <= this.visibleToasts())),
          (this.toastType = C(() => this.toast().type ?? "default")),
          (this.toastClass = C(() => this.toast().class ?? "")),
          (this.toastPosition = C(
            () => this.toast().position ?? this.position(),
          )),
          (this.toastDescriptionClass = C(
            () => this.toast().descriptionClass ?? "",
          )),
          (this.heightIndex = C(() =>
            this.heights().findIndex((e) => e.toastId === this.toast().id),
          )),
          (this.offset = xe({
            source: () => ({
              heightIndex: this.heightIndex(),
              toastsHeightBefore: this.toastsHeightBefore(),
            }),
            computation: ({ heightIndex: e, toastsHeightBefore: n }) =>
              Math.round(e * pt + n),
          })),
          (this.closeTimerStartTimeRef = 0),
          (this.lastCloseTimerStartTimeRef = 0),
          (this.pointerStartRef = null),
          (this.coords = C(() => this.toastPosition().split("-"))),
          (this.toastsHeightBefore = C(() =>
            this.heights().reduce(
              (e, n, i) => (i >= this.heightIndex() ? e : e + n.height),
              0,
            ),
          )),
          (this.invert = C(() => this.toast().invert ?? this._invert())),
          (this.closeButton = C(
            () => this.toast().closeButton ?? this._closeButton(),
          )),
          (this.disabled = C(() => this.toastType() === "loading")),
          (this.remainingTime = 0),
          (this.isPromiseLoadingOrInfiniteDuration = C(
            () =>
              (this.toast().promise && this.toastType() === "loading") ||
              this.toast().duration === Number.POSITIVE_INFINITY,
          )),
          (this.toastClasses = C(() =>
            lt(
              this._class(),
              this.toastClass(),
              this.classes().toast,
              this.toast().classes?.toast,
              this.classes()[this.toastType()],
              this.toast().classes?.[this.toastType()],
            ),
          )),
          (this.toastStyle = C(() =>
            T(
              {
                "--index": `${this.index()}`,
                "--toasts-before": `${this.index()}`,
                "--z-index": `${this.toasts().length - this.index()}`,
                "--offset": `${this.removed() ? this.offsetBeforeRemove() : this.offset()}px`,
                "--initial-height": this.expandByDefault()
                  ? "auto"
                  : `${this.initialHeight()}px`,
              },
              this._style(),
            ),
          )),
          de(() => {
            this.toast().updated &&
              (clearTimeout(this.timeoutId),
              (this.remainingTime =
                this.toast().duration ?? this.duration() ?? we),
              this.startTimer());
          }),
          Ze((e) => {
            (this.isPromiseLoadingOrInfiniteDuration() ||
              (this.expanded() || this.interacting()
                ? this.pauseTimer()
                : this.startTimer()),
              e(() => clearTimeout(this.timeoutId)));
          }),
          de(() => {
            this.toast().delete && this.deleteToast();
          }));
      }
      ngAfterViewInit() {
        ((this.remainingTime = this.toast().duration ?? this.duration() ?? we),
          this.mounted.set(!0));
        let e = this.toastRef().nativeElement.getBoundingClientRect().height;
        (this.initialHeight.set(e),
          this.addHeight({ toastId: this.toast().id, height: e }));
      }
      ngOnDestroy() {
        (clearTimeout(this.timeoutId), this.removeHeight(this.toast().id));
      }
      deleteToast() {
        (this.removed.set(!0),
          this.offsetBeforeRemove.set(this.offset()),
          this.removeHeight(this.toast().id),
          setTimeout(() => {
            this.dismiss(this.toast().id);
          }, Mn));
      }
      pauseTimer() {
        if (this.lastCloseTimerStartTimeRef < this.closeTimerStartTimeRef) {
          let e = new Date().getTime() - this.closeTimerStartTimeRef;
          this.remainingTime = this.remainingTime - e;
        }
        this.lastCloseTimerStartTimeRef = new Date().getTime();
      }
      startTimer() {
        ((this.closeTimerStartTimeRef = new Date().getTime()),
          (this.timeoutId = setTimeout(() => {
            (this.toast().onAutoClose?.(this.toast()), this.deleteToast());
          }, this.remainingTime)));
      }
      onPointerDown(e) {
        if (this.disabled() || !this.toast().dismissible) return;
        this.offsetBeforeRemove.set(this.offset());
        let n = e.target;
        (n.setPointerCapture(e.pointerId),
          n.tagName !== "BUTTON" &&
            (this.swiping.set(!0),
            (this.pointerStartRef = { x: e.clientX, y: e.clientY })));
      }
      onPointerUp() {
        if (this.swipeOut() || !this.toast().dismissible) return;
        this.pointerStartRef = null;
        let e = Number(
          this.toastRef()
            .nativeElement.style.getPropertyValue("--swipe-amount")
            .replace("px", "") || 0,
        );
        if (Math.abs(e) >= Pn) {
          (this.offsetBeforeRemove.set(this.offset()),
            this.toast().onDismiss?.(this.toast()),
            this.deleteToast(),
            this.swipeOut.set(!0));
          return;
        }
        (this.toastRef().nativeElement.style.setProperty(
          "--swipe-amount",
          "0px",
        ),
          this.swiping.set(!1));
      }
      onPointerMove(e) {
        if (!this.pointerStartRef || !this.toast().dismissible) return;
        let n = e.clientY - this.pointerStartRef.y,
          i = e.clientX - this.pointerStartRef.x,
          d = (this.coords()[0] === "top" ? Math.min : Math.max)(0, n),
          b = e.pointerType === "touch" ? 10 : 2;
        Math.abs(d) > b
          ? this.toastRef().nativeElement.style.setProperty(
              "--swipe-amount",
              `${n}px`,
            )
          : Math.abs(i) > b && (this.pointerStartRef = null);
      }
      onCloseButtonClick() {
        this.disabled() ||
          !this.toast().dismissible ||
          (this.deleteToast(), this.toast().onDismiss?.(this.toast()));
      }
      onCancelClick() {
        let e = this.toast();
        e.dismissible &&
          (this.deleteToast(), e.cancel?.onClick && e.cancel.onClick());
      }
      onActionClick(e) {
        (this.toast().action?.onClick(e),
          !e.defaultPrevented && this.deleteToast());
      }
      static {
        this.ɵfac = function (n) {
          return new (n || t)();
        };
      }
      static {
        this.ɵcmp = E({
          type: t,
          selectors: [["ngx-sonner-toast"]],
          viewQuery: function (n, i) {
            (n & 1 && Pe(i.toastRef, Vt, 5), n & 2 && Me());
          },
          inputs: {
            toast: [1, "toast"],
            index: [1, "index"],
            expanded: [1, "expanded"],
            _invert: [1, "invert", "_invert"],
            position: [1, "position"],
            visibleToasts: [1, "visibleToasts"],
            expandByDefault: [1, "expandByDefault"],
            _closeButton: [1, "closeButton", "_closeButton"],
            interacting: [1, "interacting"],
            cancelButtonStyle: [1, "cancelButtonStyle"],
            actionButtonStyle: [1, "actionButtonStyle"],
            duration: [1, "duration"],
            descriptionClass: [1, "descriptionClass"],
            _classes: [1, "classes", "_classes"],
            unstyled: [1, "unstyled"],
            _class: [1, "class", "_class"],
            _style: [1, "style", "_style"],
          },
          ngContentSelectors: dt,
          decls: 5,
          vars: 22,
          consts: [
            ["toastRef", ""],
            [
              "data-sonner-toast",
              "",
              "aria-atomic",
              "true",
              "role",
              "status",
              "tabindex",
              "0",
              3,
              "pointerdown",
              "pointerup",
              "pointermove",
            ],
            ["aria-label", "Close toast", "data-close-button", "", 3, "class"],
            ["aria-label", "Close toast", "data-close-button", "", 3, "click"],
            [
              "xmlns",
              "http://www.w3.org/2000/svg",
              "width",
              "12",
              "height",
              "12",
              "viewBox",
              "0 0 24 24",
              "fill",
              "none",
              "stroke",
              "currentColor",
              "stroke-width",
              "1.5",
              "stroke-linecap",
              "round",
              "stroke-linejoin",
              "round",
            ],
            ["x1", "18", "y1", "6", "x2", "6", "y2", "18"],
            ["x1", "6", "y1", "6", "x2", "18", "y2", "18"],
            [4, "ngComponentOutlet", "ngComponentOutletInputs"],
            ["data-icon", ""],
            ["data-content", ""],
            ["data-title", "", 3, "class"],
            ["data-description", "", 3, "class"],
            ["data-button", "", "data-cancel", "", 3, "style", "class"],
            ["data-button", "", 3, "style", "class"],
            ["data-title", ""],
            ["data-description", ""],
            ["data-button", "", "data-cancel", "", 3, "click"],
            ["data-button", "", 3, "click"],
          ],
          template: function (n, i) {
            if (n & 1) {
              let r = F();
              (ce(ct),
                a(0, "li", 1, 0),
                y("pointerdown", function (b) {
                  return (f(r), _(i.onPointerDown(b)));
                })("pointerup", function () {
                  return (f(r), _(i.onPointerUp()));
                })("pointermove", function (b) {
                  return (f(r), _(i.onPointerMove(b)));
                }),
                x(2, $t, 4, 3, "button", 2)(3, jt, 2, 4, "ng-container")(
                  4,
                  cn,
                  6,
                  5,
                ),
                s());
            }
            n & 2 &&
              (se(i.toastStyle()),
              H(i.toastClasses()),
              Z("aria-live", i.toast().important ? "assertive" : "polite")(
                "data-styled",
                !(i.toast().component || i.toast().unstyled || i.unstyled()),
              )("data-mounted", i.mounted())(
                "data-promise",
                !!i.toast().promise,
              )("data-removed", i.removed())("data-visible", i.isVisible())(
                "data-y-position",
                i.coords()[0],
              )("data-x-position", i.coords()[1])("data-index", i.index())(
                "data-front",
                i.isFront(),
              )("data-swiping", i.swiping())(
                "data-dismissible",
                i.toast().dismissible,
              )("data-type", i.toastType())("data-invert", i.invert())(
                "data-swipe-out",
                i.swipeOut(),
              )(
                "data-expanded",
                i.expanded() || (i.expandByDefault() && i.mounted()),
              ),
              l(2),
              w(i.closeButton() && !i.toast().component ? 2 : -1),
              l(),
              w(i.toast().component ? 3 : 4));
          },
          dependencies: [et, In, Nn],
          encapsulation: 2,
          changeDetection: 0,
        });
      }
    }
    return t;
  })(),
  Ni = (() => {
    class t {
      constructor() {
        ((this.platformId = A(Y)),
          (this.toasts = P.toasts),
          (this.heights = P.heights),
          (this.reset = P.reset),
          (this.invert = u(!1, { transform: ie })),
          (this.theme = u("light")),
          (this.position = u("bottom-right")),
          (this.hotKey = u(["altKey", "KeyT"])),
          (this.richColors = u(!1, { transform: ie })),
          (this.expand = u(!1, { transform: ie })),
          (this.duration = u(we, { transform: Oe })),
          (this.visibleToasts = u(Tn, { transform: Oe })),
          (this.closeButton = u(!1, { transform: ie })),
          (this.toastOptions = u({})),
          (this.offset = u(null)),
          (this.dir = u(this.getDocumentDirection())),
          (this._class = u("", { alias: "class" })),
          (this._style = u({}, { alias: "style" })),
          (this.possiblePositions = C(() =>
            Array.from(
              new Set(
                [
                  this.position(),
                  ...this.toasts()
                    .filter((e) => e.position)
                    .map((e) => e.position),
                ].filter(Boolean),
              ),
            ),
          )),
          (this.expanded = xe({
            source: this.toasts,
            computation: (e) => e.length < 1,
          })),
          (this.actualTheme = xe({
            source: this.theme,
            computation: (e) => this.getActualTheme(e),
          })),
          (this.interacting = k(!1)),
          (this.listRef = Ae("listRef")),
          (this.lastFocusedElementRef = k(null)),
          (this.isFocusWithinRef = k(!1)),
          (this.hotKeyLabel = C(() =>
            this.hotKey().join("+").replace(/Key/g, "").replace(/Digit/g, ""),
          )),
          (this.toasterStyles = C(() =>
            T(
              {
                "--front-toast-height": `${this.heights()[0]?.height}px`,
                "--offset":
                  typeof this.offset() == "number"
                    ? `${this.offset()}px`
                    : (this.offset() ?? `${En}`),
                "--width": `${An}px`,
                "--gap": `${pt}px`,
              },
              this._style(),
            ),
          )),
          (this.handleKeydown = (e) => {
            let n = this.listRef()?.nativeElement;
            if (!n) return;
            (this.hotKey().every((r) => e[r] || e.code === r) &&
              (this.expanded.set(!0), n.focus()),
              e.code === "Escape" &&
                (document.activeElement === n ||
                  n.contains(document.activeElement)) &&
                this.expanded.set(!1));
          }),
          (this.handleThemePreferenceChange = ({ matches: e }) => {
            this.theme() === "system" &&
              this.actualTheme.set(e ? "dark" : "light");
          }),
          this.reset(),
          z(this.platformId) &&
            (document.addEventListener("keydown", this.handleKeydown),
            window
              .matchMedia("(prefers-color-scheme: dark)")
              .addEventListener("change", this.handleThemePreferenceChange)));
      }
      ngOnDestroy() {
        z(this.platformId) &&
          (document.removeEventListener("keydown", this.handleKeydown),
          window
            .matchMedia("(prefers-color-scheme: dark)")
            .removeEventListener("change", this.handleThemePreferenceChange));
      }
      handleBlur(e) {
        this.isFocusWithinRef() &&
          !e.target.contains(e.relatedTarget) &&
          (this.isFocusWithinRef.set(!1),
          this.lastFocusedElementRef() &&
            (this.lastFocusedElementRef()?.focus({ preventScroll: !0 }),
            this.lastFocusedElementRef.set(null)));
      }
      handleFocus(e) {
        (e.target instanceof HTMLElement &&
          e.target.dataset.dismissible === "false") ||
          this.isFocusWithinRef() ||
          (this.isFocusWithinRef.set(!0),
          this.lastFocusedElementRef.set(e.relatedTarget));
      }
      handlePointerDown(e) {
        (e.target instanceof HTMLElement &&
          e.target.dataset.dismissible === "false") ||
          this.interacting.set(!0);
      }
      handleMouseLeave() {
        this.interacting() || this.expanded.set(!1);
      }
      getActualTheme(e) {
        return e !== "system"
          ? e
          : z(this.platformId) &&
              window.matchMedia?.("(prefers-color-scheme: dark)").matches
            ? "dark"
            : "light";
      }
      getDocumentDirection() {
        if (typeof window > "u" || typeof document > "u") return "ltr";
        let e = document.documentElement.getAttribute("dir");
        return !e || e === "auto"
          ? window.getComputedStyle(document.documentElement).direction
          : e;
      }
      static {
        this.ɵfac = function (n) {
          return new (n || t)();
        };
      }
      static {
        this.ɵcmp = E({
          type: t,
          selectors: [["ngx-sonner-toaster"]],
          viewQuery: function (n, i) {
            (n & 1 && Pe(i.listRef, dn, 5), n & 2 && Me());
          },
          inputs: {
            invert: [1, "invert"],
            theme: [1, "theme"],
            position: [1, "position"],
            hotKey: [1, "hotKey"],
            richColors: [1, "richColors"],
            expand: [1, "expand"],
            duration: [1, "duration"],
            visibleToasts: [1, "visibleToasts"],
            closeButton: [1, "closeButton"],
            toastOptions: [1, "toastOptions"],
            offset: [1, "offset"],
            dir: [1, "dir"],
            _class: [1, "class", "_class"],
            _style: [1, "style", "_style"],
          },
          ngContentSelectors: dt,
          decls: 1,
          vars: 1,
          consts: [
            ["listRef", ""],
            [3, "tabIndex"],
            ["data-sonner-toaster", "", 3, "tabIndex", "class", "style"],
            [
              "data-sonner-toaster",
              "",
              3,
              "blur",
              "focus",
              "mouseenter",
              "mousemove",
              "mouseleave",
              "pointerdown",
              "pointerup",
              "tabIndex",
            ],
            [
              3,
              "index",
              "toast",
              "invert",
              "visibleToasts",
              "closeButton",
              "interacting",
              "position",
              "expandByDefault",
              "expanded",
              "actionButtonStyle",
              "cancelButtonStyle",
              "class",
              "descriptionClass",
              "classes",
              "duration",
              "unstyled",
            ],
            [
              3,
              "index",
              "toast",
              "invert",
              "visibleToasts",
              "closeButton",
              "interacting",
              "position",
              "expandByDefault",
              "expanded",
              "actionButtonStyle",
              "cancelButtonStyle",
              "descriptionClass",
              "classes",
              "duration",
              "unstyled",
            ],
            [3, "isVisible"],
            ["type", "success"],
            ["type", "error"],
            ["type", "warning"],
            ["type", "info"],
          ],
          template: function (n, i) {
            (n & 1 && (ce(ct), x(0, xn, 3, 2, "section", 1)),
              n & 2 && w(i.toasts().length > 0 ? 0 : -1));
          },
          dependencies: [Rn, On, Sn, Fn],
          styles: [
            `html[dir=ltr],[data-sonner-toaster][dir=ltr]{--toast-icon-margin-start: var(--ngx-sonner-toast-icon-margin-start, -3px);--toast-icon-margin-end: var(--ngx-sonner-toast-icon-margin-end, 4px);--toast-svg-margin-start: var(--ngx-sonner-toast-svg-margin-start,-1px);--toast-svg-margin-end: var(--ngx-sonner-toast-svg-margin-end, 0px);--toast-button-margin-start: var(--ngx-sonner-toast-button-margin-start, auto);--toast-button-margin-end: var(--ngx-sonner-toast-button-margin-end, 0);--toast-close-button-start: var(--ngx-sonner-toast-close-button-start, 0);--toast-close-button-end: var(--ngx-sonner-toast-close-button-end, unset);--toast-close-button-transform: var(--ngx-sonner-toast-close-button-transform, translate(-35%, -35%))}html[dir=rtl],[data-sonner-toaster][dir=rtl]{--toast-icon-margin-start: var(--ngx-sonner-rtl-toast-icon-margin-start, 4px);--toast-icon-margin-end: var(--ngx-sonner-rtl-toast-icon-margin-end, -3px);--toast-svg-margin-start: var(--ngx-sonner-rtl-toast-svg-margin-start, 0px);--toast-svg-margin-end: var(--ngx-sonner-rtl-toast-svg-margin-end, -1px);--toast-button-margin-start: var(--ngx-sonner-rtl-toast-button-margin-start, 0);--toast-button-margin-end: var(--ngx-sonner-rtl-toast-button-margin-end, auto);--toast-close-button-start: var(--ngx-sonner-rtl-toast-close-button-start, unset);--toast-close-button-end: var(--ngx-sonner-rtl-toast-close-button-end, 0);--toast-close-button-transform: var(--ngx-sonner-rtl-toast-close-button-transform, translate(35%, -35%))}[data-sonner-toaster]{position:fixed;width:var(--width);font-family:var(--ngx-sonner-font-family, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, Segoe UI, Roboto, Helvetica Neue, Arial, Noto Sans, sans-serif, Apple Color Emoji, Segoe UI Emoji, Segoe UI Symbol, Noto Color Emoji);--gray1: hsl(0, 0%, 99%);--gray2: hsl(0, 0%, 97.3%);--gray3: hsl(0, 0%, 95.1%);--gray4: hsl(0, 0%, 93%);--gray5: hsl(0, 0%, 90.9%);--gray6: hsl(0, 0%, 88.7%);--gray7: hsl(0, 0%, 85.8%);--gray8: hsl(0, 0%, 78%);--gray9: hsl(0, 0%, 56.1%);--gray10: hsl(0, 0%, 52.3%);--gray11: hsl(0, 0%, 43.5%);--gray12: hsl(0, 0%, 9%);--border-radius: var(--ngx-sonner-border-radius, 8px);box-sizing:border-box;padding:0;margin:0;list-style:none;outline:none;z-index:999999999}[data-sonner-toaster][data-x-position=right]{right:max(var(--offset),env(safe-area-inset-right))}[data-sonner-toaster][data-x-position=left]{left:max(var(--offset),env(safe-area-inset-left))}[data-sonner-toaster][data-x-position=center]{left:50%;transform:translate(-50%)}[data-sonner-toaster][data-y-position=top]{top:max(var(--offset),env(safe-area-inset-top))}[data-sonner-toaster][data-y-position=bottom]{bottom:max(var(--offset),env(safe-area-inset-bottom))}[data-sonner-toast]{--y: translateY(100%);--lift-amount: calc(var(--lift) * var(--gap));z-index:var(--z-index);position:absolute;opacity:0;transform:var(--y);filter:blur(0);touch-action:none;transition:transform .4s,opacity .4s,height .4s,box-shadow .2s;box-sizing:border-box;outline:none;overflow-wrap:anywhere}[data-sonner-toast][data-styled=true]{padding:16px;background:var(--normal-bg);border:1px solid var(--normal-border);color:var(--normal-text);border-radius:var(--border-radius);box-shadow:0 4px 12px #0000001a;width:var(--width);font-size:13px;display:flex;align-items:center;gap:6px}[data-sonner-toast]:focus-visible{box-shadow:0 4px 12px #0000001a,0 0 0 2px #0003}[data-sonner-toast][data-y-position=top]{top:0;--y: translateY(-100%);--lift: 1;--lift-amount: calc(1 * var(--gap))}[data-sonner-toast][data-y-position=bottom]{bottom:0;--y: translateY(100%);--lift: -1;--lift-amount: calc(var(--lift) * var(--gap))}[data-sonner-toast] [data-description]{font-weight:400;line-height:1.4;color:inherit}[data-sonner-toast] [data-title]{font-weight:500;line-height:1.5;color:inherit}[data-sonner-toast] [data-icon]{display:flex;height:16px;width:16px;position:relative;justify-content:flex-start;align-items:center;flex-shrink:0;margin-left:var(--toast-icon-margin-start);margin-right:var(--toast-icon-margin-end)}[data-sonner-toast][data-promise=true] [data-icon]>svg{opacity:0;transform:scale(.8);transform-origin:center;animation:sonner-fade-in .3s ease forwards}[data-sonner-toast] [data-icon]>*{flex-shrink:0}[data-sonner-toast] [data-icon] svg{margin-left:var(--toast-svg-margin-start);margin-right:var(--toast-svg-margin-end)}[data-sonner-toast] [data-content]{display:flex;flex-direction:column;gap:2px}[data-sonner-toast] [data-button]{border-radius:4px;padding-left:8px;padding-right:8px;height:24px;font-size:12px;color:var(--normal-bg);background:var(--normal-text);margin-left:var(--toast-button-margin-start);margin-right:var(--toast-button-margin-end);border:none;cursor:pointer;outline:none;display:flex;align-items:center;flex-shrink:0;transition:opacity .4s,box-shadow .2s}[data-sonner-toast] [data-button]:focus-visible{box-shadow:var(--ngx-sonner-toast-focus-box-shadow, 0 0 0 2px rgba(0, 0, 0, .4))}[data-sonner-toast] [data-button]:first-of-type{margin-left:var(--toast-button-margin-start);margin-right:var(--toast-button-margin-end)}[data-sonner-toast] [data-cancel]{color:var(--normal-text);background:#00000014}[data-sonner-toast][data-theme=dark] [data-cancel]{background:#ffffff4d}[data-sonner-toast] [data-close-button]{position:absolute;left:var(--toast-close-button-start);right:var(--toast-close-button-end);top:0;height:20px;width:20px;display:flex;justify-content:center;align-items:center;padding:0;background:var(--ngx-sonner-toast-close-button-background, var(--gray1));color:var(--ngx-sonner-toast-close-button-color, var(--gray12));border:var(--ngx-sonner-toast-close-button-border, 1px solid var(--gray4));transform:var(--toast-close-button-transform);border-radius:50%;cursor:pointer;z-index:1;transition:opacity .1s,background .2s,border-color .2s}[data-sonner-toast] [data-close-button]:focus-visible{box-shadow:0 4px 12px #0000001a,0 0 0 2px #0003}[data-sonner-toast] [data-disabled=true]{cursor:not-allowed}[data-sonner-toast]:hover [data-close-button]:hover{background:var(--ngx-sonner-toast-close-button-hover-background, var(--gray2));color:var(--ngx-sonner-toast-close-button-hover-color, var(--gray12));border-color:var(--ngx-sonner-toast-close-button-hover-border-color, var(--gray5))}[data-sonner-toast][data-swiping=true]:before{content:"";position:absolute;left:0;right:0;height:100%;z-index:-1}[data-sonner-toast][data-y-position=top][data-swiping=true]:before{bottom:50%;transform:scaleY(3) translateY(50%)}[data-sonner-toast][data-y-position=bottom][data-swiping=true]:before{top:50%;transform:scaleY(3) translateY(-50%)}[data-sonner-toast][data-swiping=false][data-removed=true]:before{content:"";position:absolute;inset:0;transform:scaleY(2)}[data-sonner-toast]:after{content:"";position:absolute;left:0;height:calc(var(--gap) + 1px);bottom:100%;width:100%}[data-sonner-toast][data-mounted=true]{--y: translateY(0);opacity:1}[data-sonner-toast][data-expanded=false][data-front=false]{--scale: var(--toasts-before) * .05 + 1;--y: translateY(calc(var(--lift-amount) * var(--toasts-before))) scale(calc(-1 * var(--scale)));height:var(--front-toast-height)}[data-sonner-toast]>*{transition:opacity .4s}[data-sonner-toast][data-expanded=false][data-front=false][data-styled=true]>*{opacity:0}[data-sonner-toast][data-visible=false]{opacity:0;pointer-events:none}[data-sonner-toast][data-mounted=true][data-expanded=true]{--y: translateY(calc(var(--lift) * var(--offset)));height:var(--initial-height)}[data-sonner-toast][data-removed=true][data-front=true][data-swipe-out=false]{--y: translateY(calc(var(--lift) * -100%));opacity:0}[data-sonner-toast][data-removed=true][data-front=false][data-swipe-out=false][data-expanded=true]{--y: translateY(calc(var(--lift) * var(--offset) + var(--lift) * -100%));opacity:0}[data-sonner-toast][data-removed=true][data-front=false][data-swipe-out=false][data-expanded=false]{--y: translateY(40%);opacity:0;transition:transform .5s,opacity .2s}[data-sonner-toast][data-removed=true][data-front=false]:before{height:calc(var(--initial-height) + 20%)}[data-sonner-toast][data-swiping=true]{transform:var(--y) translateY(var(--swipe-amount, 0px));transition:none}[data-sonner-toast][data-swipe-out=true][data-y-position=bottom],[data-sonner-toast][data-swipe-out=true][data-y-position=top]{animation:swipe-out .2s ease-out forwards}@keyframes swipe-out{0%{transform:translateY(calc(var(--lift) * var(--offset) + var(--swipe-amount)));opacity:1}to{transform:translateY(calc(var(--lift) * var(--offset) + var(--swipe-amount) + var(--lift) * -100%));opacity:0}}@media (max-width: 600px){[data-sonner-toaster]{position:fixed;--mobile-offset: 16px;right:var(--mobile-offset);left:var(--mobile-offset);width:100%}[data-sonner-toaster] [data-sonner-toast]{left:0;right:0;width:calc(100% - 32px)}[data-sonner-toaster][data-x-position=left]{left:var(--mobile-offset)}[data-sonner-toaster][data-y-position=bottom]{bottom:20px}[data-sonner-toaster][data-y-position=top]{top:20px}[data-sonner-toaster][data-x-position=center]{left:var(--mobile-offset);right:var(--mobile-offset);transform:none}}[data-sonner-toaster][data-theme=light]{--normal-bg: var(--ngx-sonner-toast-normal-background, #fff);--normal-border: var(--ngx-sonner-toast-normal-border-color, var(--gray4));--normal-text: var(--ngx-sonner-toast-normal-color, var(--gray12));--success-bg: var(--ngx-sonner-toast-success-background, hsl(143, 85%, 96%));--success-border: var(--ngx-sonner-toast-success-border, hsl(145, 92%, 91%));--success-text: var(--ngx-sonner-toast-success-color, hsl(140, 100%, 27%));--info-bg: var(--ngx-sonner-toast-info-background, hsl(208, 100%, 97%));--info-border: var(--ngx-sonner-toast-info-border, hsl(221, 91%, 91%));--info-text: var(--ngx-sonner-toast-info-color, hsl(210, 92%, 45%));--warning-bg: var(--ngx-sonner-toast-warning-background, hsl(49, 100%, 97%));--warning-border: var(--ngx-sonner-toast-warning-border, hsl(49, 91%, 91%));--warning-text: var(--ngx-sonner-toast-warning-color, hsl(31, 92%, 45%));--error-bg: var(--ngx-sonner-toast-error-background, hsl(359, 100%, 97%));--error-border: var(--ngx-sonner-toast-error-border, hsl(359, 100%, 94%));--error-text: var(--ngx-sonner-toast-error-color, hsl(360, 100%, 45%))}[data-sonner-toaster][data-theme=light] [data-sonner-toast][data-invert=true]{--normal-bg: var(--ngx-sonner-toast-inverse-normal-background, #000);--normal-border: var(--ngx-sonner-toast-inverse-normal-border-color, hsl(0, 0%, 20%));--normal-text: var(--ngx-sonner-toast-inverse-normal-color, var(--gray1))}[data-sonner-toaster][data-theme=dark] [data-sonner-toast][data-invert=true]{--normal-bg: var(--ngx-sonner-toast-inverse-dark-normal-background, #fff);--normal-border: var(--ngx-sonner-toast-inverse-dark-normal-border-color, var(--gray3));--normal-text: var(--ngx-sonner-toast-inverse-dark-normal-color, var(--gray12))}[data-sonner-toaster][data-theme=dark]{--normal-bg: var(--ngx-sonner-toast-dark-normal-background, #000);--normal-border: var(--ngx-sonner-toast-dark-normal-border-color, hsl(0, 0%, 20%));--normal-text: var(--ngx-sonner-toast-dark-normal-color, var(--gray1));--success-bg: var(--ngx-sonner-toast-dark-success-background, hsl(150, 100%, 6%));--success-border: var(--ngx-sonner-toast-dark-success-border, hsl(147, 100%, 12%));--success-text: var(--ngx-sonner-toast-dark-success-color, hsl(150, 86%, 65%));--info-bg: var(--ngx-sonner-toast-dark-info-background, hsl(215, 100%, 6%));--info-border: var(--ngx-sonner-toast-dark-info-border, hsl(223, 100%, 12%));--info-text: var(--ngx-sonner-toast-dark-info-color, hsl(216, 87%, 65%));--warning-bg: var(--ngx-sonner-toast-dark-warning-background, hsl(64, 100%, 6%));--warning-border: var(--ngx-sonner-toast-dark-warning-border, hsl(60, 100%, 12%));--warning-text: var(--ngx-sonner-toast-dark-warning-color, hsl(46, 87%, 65%));--error-bg: var(--ngx-sonner-toast-dark-error-background, hsl(358, 76%, 10%));--error-border: var(--ngx-sonner-toast-dark-error-border, hsl(357, 89%, 16%));--error-text: var(--ngx-sonner-toast-dark-error-color, hsl(358, 100%, 81%))}[data-rich-colors=true] [data-sonner-toast][data-type=success],[data-rich-colors=true] [data-sonner-toast][data-type=success] [data-close-button]{background:var(--success-bg);border-color:var(--success-border);color:var(--success-text)}[data-rich-colors=true] [data-sonner-toast][data-type=info],[data-rich-colors=true] [data-sonner-toast][data-type=info] [data-close-button]{background:var(--info-bg);border-color:var(--info-border);color:var(--info-text)}[data-rich-colors=true] [data-sonner-toast][data-type=warning],[data-rich-colors=true] [data-sonner-toast][data-type=warning] [data-close-button]{background:var(--warning-bg);border-color:var(--warning-border);color:var(--warning-text)}[data-rich-colors=true] [data-sonner-toast][data-type=error],[data-rich-colors=true] [data-sonner-toast][data-type=error] [data-close-button]{background:var(--error-bg);border-color:var(--error-border);color:var(--error-text)}.sonner-loading-wrapper{--size: 16px;height:var(--size);width:var(--size);position:absolute;inset:0;z-index:10}.sonner-loading-wrapper[data-visible=false]{transform-origin:center;animation:sonner-fade-out .2s ease forwards}.sonner-spinner{position:relative;top:50%;left:50%;height:var(--size);width:var(--size)}.sonner-loading-bar{animation:sonner-spin 1.2s linear infinite;background:var(--gray11);border-radius:6px;height:8%;left:-10%;position:absolute;top:-3.9%;width:24%}.sonner-loading-bar:nth-child(1){animation-delay:-1.2s;transform:rotate(.0001deg) translate(146%)}.sonner-loading-bar:nth-child(2){animation-delay:-1.1s;transform:rotate(30deg) translate(146%)}.sonner-loading-bar:nth-child(3){animation-delay:-1s;transform:rotate(60deg) translate(146%)}.sonner-loading-bar:nth-child(4){animation-delay:-.9s;transform:rotate(90deg) translate(146%)}.sonner-loading-bar:nth-child(5){animation-delay:-.8s;transform:rotate(120deg) translate(146%)}.sonner-loading-bar:nth-child(6){animation-delay:-.7s;transform:rotate(150deg) translate(146%)}.sonner-loading-bar:nth-child(7){animation-delay:-.6s;transform:rotate(180deg) translate(146%)}.sonner-loading-bar:nth-child(8){animation-delay:-.5s;transform:rotate(210deg) translate(146%)}.sonner-loading-bar:nth-child(9){animation-delay:-.4s;transform:rotate(240deg) translate(146%)}.sonner-loading-bar:nth-child(10){animation-delay:-.3s;transform:rotate(270deg) translate(146%)}.sonner-loading-bar:nth-child(11){animation-delay:-.2s;transform:rotate(300deg) translate(146%)}.sonner-loading-bar:nth-child(12){animation-delay:-.1s;transform:rotate(330deg) translate(146%)}@keyframes sonner-fade-in{0%{opacity:0;transform:scale(.8)}to{opacity:1;transform:scale(1)}}@keyframes sonner-fade-out{0%{opacity:1;transform:scale(1)}to{opacity:0;transform:scale(.8)}}@keyframes sonner-spin{0%{opacity:1}to{opacity:.15}}@media (prefers-reduced-motion){[data-sonner-toast],[data-sonner-toast]>*,.sonner-loading-bar{transition:none!important;animation:none!important}}.sonner-loader{position:absolute;top:50%;left:50%;transform:translate(-50%,-50%);transform-origin:center;transition:opacity .2s,transform .2s}.sonner-loader[data-visible=false]{opacity:0;transform:scale(.8) translate(-50%,-50%)}
`,
          ],
          encapsulation: 2,
          changeDetection: 0,
        });
      }
    }
    return t;
  })();
var Bn = (t, o) => o.label;
function Dn(t, o) {
  if ((t & 1 && (a(0, "a", 1), v(1), R(2, "uppercase"), s()), t & 2)) {
    let e = o.$implicit,
      n = c();
    (h("ngClass", n.variant)("routerLink", e.routerLink || "./")(
      "fragment",
      e.anchor,
    ),
      l(),
      ne(V(2, 4, e.label)));
  }
}
var ut = (() => {
  class t {
    navbarLinks;
    variant = "primary";
    static ɵfac = function (n) {
      return new (n || t)();
    };
    static ɵcmp = E({
      type: t,
      selectors: [["pool-land-navbar"]],
      inputs: { navbarLinks: "navbarLinks", variant: "variant" },
      decls: 3,
      vars: 0,
      consts: [
        [
          1,
          "border-secondary-green",
          "hidden",
          "justify-around",
          "gap-4",
          "text-nowrap",
          "rounded-full",
          "border-2",
          "bg-white",
          "px-4",
          "lg:flex",
          "xl:gap-8",
        ],
        [
          "queryParamsHandling",
          "preserve",
          1,
          "header-nav__item",
          "font-medium",
          3,
          "ngClass",
          "routerLink",
          "fragment",
        ],
      ],
      template: function (n, i) {
        (n & 1 && (a(0, "nav", 0), D(1, Dn, 3, 6, "a", 1, Bn), s()),
          n & 2 && (l(), L(i.navbarLinks)));
      },
      dependencies: [N, j, nt, oe],
      styles: [
        ".header-nav__item[_ngcontent-%COMP%]{cursor:pointer;border-radius:9999px;padding:.5rem;font-weight:400;--tw-text-opacity: 1;color:rgb(0 0 0 / var(--tw-text-opacity, 1))}.header-nav__item[_ngcontent-%COMP%]:hover{background-color:rgb(255 255 255 / var(--tw-bg-opacity, 1));--tw-bg-opacity: .4 }.secondary[_ngcontent-%COMP%]{--tw-text-opacity: 1;color:rgb(152 251 150 / var(--tw-text-opacity, 1))}",
      ],
      changeDetection: 0,
    });
  }
  return t;
})();
var Ln = [[["pool-hero-button"]]],
  Vn = ["pool-hero-button"];
function $n(t, o) {
  t & 1 && (a(0, "div", 3), I(1), s());
}
var Wi = (() => {
  class t {
    platformId = A(Y);
    cdr = A(K);
    navbarLinks;
    isMobile = !0;
    onWindowResize() {
      this.isMobile = window.innerWidth < 640;
    }
    ngAfterViewInit() {
      z(this.platformId) &&
        ((this.isMobile = window.innerWidth < 640), this.cdr.detectChanges());
    }
    static ɵfac = function (n) {
      return new (n || t)();
    };
    static ɵcmp = E({
      type: t,
      selectors: [["pool-land-header"]],
      hostBindings: function (n, i) {
        n & 1 &&
          y(
            "resize",
            function (d) {
              return i.onWindowResize(d);
            },
            !1,
            Ue,
          );
      },
      inputs: { navbarLinks: "navbarLinks" },
      ngContentSelectors: Vn,
      decls: 4,
      vars: 2,
      consts: [
        [
          1,
          "relative",
          "grid",
          "grid-cols-[1fr_max-content_1fr]",
          "content-center",
          "items-center",
          "justify-items-center",
          "gap-4",
          "px-6",
          "py-4",
          "lg:px-[48px]",
          "lg:py-[16px]",
          "xl:px-[72px]",
          "xl:py-[32px]",
        ],
        [
          "routerLink",
          "/",
          "queryParamsHandling",
          "preserve",
          "src",
          "/svg/logo.svg",
          "alt",
          "Logo",
          1,
          "h-9",
          "cursor-pointer",
          "justify-self-start",
        ],
        [1, "ml-auto", "sm:ml-0", 3, "navbarLinks"],
        [
          1,
          "absolute",
          "right-20",
          "top-1/2",
          "flex",
          "-translate-y-1/2",
          "items-center",
          "gap-4",
          "sm:right-16",
          "lg:right-[56px]",
          "xl:right-[80px]",
        ],
      ],
      template: function (n, i) {
        (n & 1 &&
          (ce(Ln),
          a(0, "header", 0),
          g(1, "img", 1)(2, "pool-land-navbar", 2),
          x(3, $n, 2, 0, "div", 3),
          s()),
          n & 2 &&
            (l(2),
            h("navbarLinks", i.navbarLinks),
            l(),
            w(i.isMobile ? -1 : 3)));
      },
      dependencies: [N, ut, oe],
      encapsulation: 2,
      changeDetection: 0,
    });
  }
  return t;
})();
function Gn(t) {
  t || (ue(Gn), (t = A(ae)));
  let o = new De((e) => t.onDestroy(e.next.bind(e)));
  return (e) => e.pipe(Ve(o));
}
function jn(t, o) {
  !o?.injector && ue(jn);
  let e = o?.injector ?? A(Ge),
    n = new Le(1),
    i = de(
      () => {
        let r;
        try {
          r = t();
        } catch (d) {
          Ne(() => n.error(d));
          return;
        }
        Ne(() => n.next(r));
      },
      { injector: e, manualCleanup: !0 },
    );
  return (
    e.get(ae).onDestroy(() => {
      (i.destroy(), n.complete());
    }),
    n.asObservable()
  );
}
function zn(t, o) {
  let e = !o?.manualCleanup;
  e && !o?.injector && ue(zn);
  let n = e ? (o?.injector?.get(ae) ?? A(ae)) : null,
    i = Hn(o?.equal),
    r;
  o?.requireSync
    ? (r = k({ kind: 0 }, { equal: i }))
    : (r = k({ kind: 1, value: o?.initialValue }, { equal: i }));
  let d = t.subscribe({
    next: (b) => r.set({ kind: 1, value: b }),
    error: (b) => {
      if (o?.rejectErrors) throw b;
      r.set({ kind: 2, error: b });
    },
  });
  if (o?.requireSync && r().kind === 0) throw new Ee(601, !1);
  return (
    n?.onDestroy(d.unsubscribe.bind(d)),
    C(
      () => {
        let b = r();
        switch (b.kind) {
          case 1:
            return b.value;
          case 2:
            throw b.error;
          case 0:
            throw new Ee(601, !1);
        }
      },
      { equal: o?.equal },
    )
  );
}
function Hn(t = Object.is) {
  return (o, e) => o.kind === 1 && e.kind === 1 && t(o.value, e.value);
}
var Un = ["slider"],
  bo = (() => {
    class t {
      platformId;
      sliderRef;
      value = 0;
      min = 0;
      max = 100;
      step = 1;
      variant = u.required();
      size = u("sm");
      variantAndSize = C(() => this.variant() + " " + this.size());
      valueChanged = je();
      constructor(e) {
        this.platformId = e;
      }
      ngAfterViewInit() {
        z(this.platformId) &&
          this.updateSliderValueAndStyle(
            this.sliderRef.nativeElement,
            this.value,
          );
      }
      refresh(e) {
        this.updateSliderValueAndStyle(this.sliderRef.nativeElement, e);
      }
      updateSliderStyle(e) {
        let n = +e.value;
        this.updateSliderValueAndStyle(e, n);
      }
      updateSliderValueAndStyle(e, n) {
        let i = n;
        this.valueChanged.emit(i);
        let r = +e.min || 0,
          d = +e.max || 100,
          b = ((i - r) / (d - r)) * 100;
        e.style.setProperty("--litters-range", `${b}%`);
      }
      static ɵfac = function (n) {
        return new (n || t)(me(Y));
      };
      static ɵcmp = E({
        type: t,
        selectors: [["pool-land-input-range-slider"]],
        viewQuery: function (n, i) {
          if ((n & 1 && _e(Un, 5), n & 2)) {
            let r;
            he((r = be())) && (i.sliderRef = r.first);
          }
        },
        inputs: {
          value: "value",
          min: "min",
          max: "max",
          step: "step",
          variant: [1, "variant"],
          size: [1, "size"],
        },
        outputs: { valueChanged: "valueChanged" },
        decls: 3,
        vars: 5,
        consts: [
          ["slider", ""],
          [1, "wrapper", 3, "ngClass"],
          [
            "type",
            "range",
            1,
            "slider",
            3,
            "input",
            "change",
            "min",
            "max",
            "step",
            "value",
          ],
        ],
        template: function (n, i) {
          if (n & 1) {
            let r = F();
            (a(0, "div", 1)(1, "input", 2, 0),
              y("input", function () {
                f(r);
                let b = ke(2);
                return _(i.updateSliderStyle(b));
              })("change", function () {
                f(r);
                let b = ke(2);
                return _(i.updateSliderStyle(b));
              }),
              s()());
          }
          n & 2 &&
            (h("ngClass", i.variantAndSize()),
            l(),
            h("min", i.min)("max", i.max)("step", i.step)("value", i.value));
        },
        dependencies: [N, j],
        styles: [
          ".wrapper[_ngcontent-%COMP%]{--slider-h: 4px;--slider-bg-color: blue;--thumb-size: 16px;--thumb-bg-color: #98fb96;--thumb-border-width: 4px;--thumb-border-color: blue}.wrapper.md[_ngcontent-%COMP%]{--slider-h: 8px;--thumb-size: 28px;--thumb-border-width: 7px}.wrapper.primary-blue-with-white[_ngcontent-%COMP%]{--thumb-bg-color: white}.wrapper.accent-green-with-white[_ngcontent-%COMP%]{--slider-bg-color: #62f7a4;--thumb-bg-color: white;--thumb-border-color: #62f7a4;--thumb-border-width: 7px}.wrapper[_ngcontent-%COMP%]   input[_ngcontent-%COMP%]{width:100%;--litters-range: 75%;appearance:none;outline:none;height:var(--slider-h);border-radius:.5vmin;background:linear-gradient(to right,var(--slider-bg-color) var(--litters-range),#e2e2e2 var(--litters-range))}.wrapper[_ngcontent-%COMP%]   input.bigger[_ngcontent-%COMP%]{height:8px}.wrapper[_ngcontent-%COMP%]   input[_ngcontent-%COMP%]::-webkit-slider-thumb{appearance:none;cursor:pointer;width:var(--thumb-size);height:var(--thumb-size);background-color:var(--thumb-bg-color);border:var(--thumb-border-width) solid var(--thumb-border-color);border-radius:50%}.wrapper[_ngcontent-%COMP%]   input[_ngcontent-%COMP%]::-moz-range-thumb{width:var(--thumb-size);height:var(--thumb-size);background-color:var(--thumb-bg-color);border:var(--thumb-border-width) solid var(--thumb-border-color);border-radius:50%;cursor:pointer}",
        ],
        changeDetection: 0,
      });
    }
    return t;
  })();
var So = (() => {
  class t {
    iconUrl = u();
    variant = u("primary-blue-with-accent-green-border");
    static ɵfac = function (n) {
      return new (n || t)();
    };
    static ɵcmp = E({
      type: t,
      selectors: [["pool-land-gradient-icon"]],
      inputs: { iconUrl: [1, "iconUrl"], variant: [1, "variant"] },
      decls: 4,
      vars: 2,
      consts: [
        [1, "flex", 3, "ngClass"],
        [
          1,
          "bg-wrapper",
          "bg-primary-blue",
          "h-10",
          "w-10",
          "origin-bottom-right",
          "rotate-[15deg]",
          "rounded-lg",
        ],
        [
          1,
          "icon-wrapper",
          "border-accent-green",
          "icon-gradient",
          "relative",
          "-ml-10",
          "flex",
          "h-10",
          "w-10",
          "items-center",
          "justify-center",
          "rounded-lg",
          "border",
          "backdrop-blur-lg",
        ],
        ["alt", "Icon", 1, "w-5", 3, "src"],
      ],
      template: function (n, i) {
        (n & 1 &&
          (a(0, "div", 0),
          g(1, "div", 1),
          a(2, "div", 2),
          g(3, "img", 3),
          s()()),
          n & 2 &&
            (h("ngClass", i.variant()), l(3), fe("src", i.iconUrl(), re)));
      },
      dependencies: [N, j],
      styles: [
        ".icon-gradient[_ngcontent-%COMP%]{background:linear-gradient(45deg,#fff9,#fff3)}.primary-blue-with-accent-green-border[_ngcontent-%COMP%]   .bg-wrapper[_ngcontent-%COMP%]{--tw-bg-opacity: 1;background-color:rgb(0 0 255 / var(--tw-bg-opacity, 1))}.primary-blue-with-accent-green-border[_ngcontent-%COMP%]   .icon-wrapper[_ngcontent-%COMP%]{--tw-border-opacity: 1;border-color:rgb(98 247 164 / var(--tw-border-opacity, 1))}.secondary-blue-with-secondary-green-border[_ngcontent-%COMP%]   .bg-wrapper[_ngcontent-%COMP%]{--tw-bg-opacity: 1;background-color:rgb(29 35 201 / var(--tw-bg-opacity, 1))}.secondary-blue-with-secondary-green-border[_ngcontent-%COMP%]   .icon-wrapper[_ngcontent-%COMP%]{--tw-border-opacity: 1;border-color:rgb(152 251 150 / var(--tw-border-opacity, 1))}",
      ],
      changeDetection: 0,
    });
  }
  return t;
})();
export {
  Oi as a,
  Ni as b,
  ei as c,
  Ce as d,
  rt as e,
  hi as f,
  Wi as g,
  Gn as h,
  jn as i,
  zn as j,
  bo as k,
  So as l,
  Si as m,
}; /**i18n:e2f94bf06bdfc8c8ab493a12299261c375fc525ae09e041ca331cb13279050ab*/
