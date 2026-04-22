import {
  $a as O,
  $b as L,
  Ba as c,
  Da as A,
  Db as le,
  Eb as se,
  F as N,
  Ha as f,
  K as C,
  Ka as M,
  L as g,
  La as I,
  Ma as t,
  Na as i,
  Oa as l,
  Pa as ne,
  Qa as te,
  Sa as E,
  Sb as v,
  U as B,
  Ua as T,
  Va as y,
  Wa as F,
  X as Q,
  Za as w,
  Zb as ue,
  _ as Z,
  _a as S,
  ab as ie,
  ac as $,
  bb as oe,
  fa as K,
  ga as a,
  ic as G,
  ja as ee,
  kb as d,
  lb as re,
  mb as R,
  nc as ce,
  pb as h,
  ra as D,
  sb as b,
  tb as _e,
  vb as Fe,
  wb as ae,
  xa as m,
} from "./chunk-A4X4NSFE.js";
var pe = (n, _, e, o) => ({
    "text-primary-blue": n,
    "text-accent-green": _,
    "text-black": e,
    "text-white": o,
  }),
  Se = (n, _) => _.value;
function Ce(n, _) {
  if ((n & 1 && (t(0, "span", 3), d(1, "|"), i()), n & 2)) {
    let e = O(2);
    A("text-muted", e.variant() === "primary")(
      "text-muted-foreground",
      e.variant() === "secondary",
    );
  }
}
function ge(n, _) {
  if (n & 1) {
    let e = E();
    (t(0, "button", 1),
      S("click", function () {
        let r = C(e).$implicit,
          u = O();
        return g(u.select(r.value));
      }),
      d(1),
      i(),
      m(2, Ce, 2, 4, "span", 2));
  }
  if (n & 2) {
    let e = _.$implicit,
      o = _.$index,
      r = _.$count,
      u = O();
    (A("font-bold", e.selected)("font-normal", !e.selected),
      c(
        "ngClass",
        _e(
          7,
          pe,
          e.selected && u.variant() === "primary",
          e.selected && u.variant() === "secondary",
          !e.selected && u.variant() === "primary",
          !e.selected && u.variant() === "secondary",
        ),
      ),
      a(),
      R(" ", e.label, " "),
      a(),
      f(o !== r - 1 ? 2 : -1));
  }
}
var k = (() => {
  class n {
    locale = N(le);
    variant = B.required();
    spaced = B(!1);
    options = Q([
      { label: "ID", value: "id", selected: this.locale.includes("id") },
      { label: "EN", value: "en", selected: this.locale.includes("en") },
    ]);
    select(e) {
      ((this.locale = e),
        (document.cookie = `user_lang=${this.locale}; path=/; max-age=2592000; SameSite=Lax`));
      let o = window.location.pathname.replace(/^\/(en|id)/, "");
      window.location.href = `/${this.locale}${o}`;
    }
    static ɵfac = function (o) {
      return new (o || n)();
    };
    static ɵcmp = D({
      type: n,
      selectors: [["pool-land-language-selector"]],
      inputs: { variant: [1, "variant"], spaced: [1, "spaced"] },
      decls: 3,
      vars: 2,
      consts: [
        [1, "flex", "items-center", "gap-2"],
        [1, "text-sm", "uppercase", 3, "click", "ngClass"],
        [
          1,
          "text-xs",
          "font-light",
          "leading-none",
          3,
          "text-muted",
          "text-muted-foreground",
        ],
        [1, "text-xs", "font-light", "leading-none"],
      ],
      template: function (o, r) {
        (o & 1 && (t(0, "div", 0), M(1, ge, 3, 12, null, null, Se), i()),
          o & 2 && (A("gap-5", r.spaced()), a(), I(r.options())));
      },
      dependencies: [L, v],
      encapsulation: 2,
      changeDetection: 0,
    });
  }
  return n;
})();
var Ne = [[["pool-hero-button"]]],
  De = ["pool-hero-button"],
  me = (n, _, e) => ({
    "translate-y-2 rotate-45": n,
    "bg-accent-green": _,
    "bg-primary-blue": e,
  }),
  Ae = (n, _, e) => ({
    "opacity-0": n,
    "bg-accent-green": _,
    "bg-primary-blue": e,
  }),
  fe = (n, _, e) => ({
    "-translate-y-2 -rotate-45": n,
    "bg-accent-green": _,
    "bg-primary-blue": e,
  }),
  de = (n, _) => _.label;
function Ee(n, _) {
  if ((n & 1 && (t(0, "a", 11), d(1), Fe(2, "uppercase"), i()), n & 2)) {
    let e = _.$implicit;
    (c("routerLink", e.routerLink || "./")("fragment", e.anchor),
      a(),
      re(ae(2, 3, e.label)));
  }
}
function Le(n, _) {
  if ((n & 1 && M(0, Ee, 3, 5, "a", 11, de), n & 2)) {
    let e = O();
    I(e.navbarLinks);
  }
}
function Me(n, _) {
  if (n & 1) {
    let e = E();
    (t(0, "button", 12),
      S("click", function () {
        C(e);
        let r = O();
        return g(r.toggleMobileMenu());
      }),
      l(1, "span", 13)(2, "span", 14)(3, "span", 13),
      i());
  }
  if (n & 2) {
    let e = O();
    (a(),
      c(
        "ngClass",
        b(3, me, e.isMobileMenuOpen, e.isMobileMenuOpen, !e.isMobileMenuOpen),
      ),
      a(),
      c(
        "ngClass",
        b(7, Ae, e.isMobileMenuOpen, e.isMobileMenuOpen, !e.isMobileMenuOpen),
      ),
      a(),
      c(
        "ngClass",
        b(11, fe, e.isMobileMenuOpen, e.isMobileMenuOpen, !e.isMobileMenuOpen),
      ));
  }
}
function Ie(n, _) {
  n & 1 && (t(0, "div", 9), oe(1), i());
}
function Te(n, _) {
  if (n & 1) {
    let e = E();
    (t(0, "a", 27),
      S("click", function () {
        C(e);
        let r = O(2);
        return g(r.closeMobileMenu());
      }),
      d(1),
      i());
  }
  if (n & 2) {
    let e = _.$implicit;
    (c("routerLink", "./")("fragment", e.anchor), a(), R(" ", e.label, " "));
  }
}
function ye(n, _) {
  if (n & 1) {
    let e = E();
    (t(0, "div", 10)(1, "div", 15),
      l(2, "img", 16)(3, "div"),
      t(4, "div", 17),
      M(5, Te, 2, 3, "a", 18, de),
      t(7, "a", 19),
      S("click", function () {
        C(e);
        let r = O();
        return g(r.closeMobileMenu());
      }),
      l(8, "pool-land-button", 20),
      i(),
      l(9, "pool-land-language-selector", 21),
      i(),
      t(10, "footer", 22)(11, "p", 23),
      T(12, 1),
      l(13, "span", 24),
      y(),
      i(),
      t(14, "h5", 25),
      F(15, 2),
      i(),
      t(16, "a", 26),
      S("click", function () {
        C(e);
        let r = O();
        return g(r.closeMobileMenu());
      }),
      F(17, 3),
      i()()()());
  }
  if (n & 2) {
    let e = O();
    (a(5), I(e.navbarLinks));
  }
}
var Ue = (() => {
  class n {
    platformId = N(Z);
    cdr = N(se);
    renderer = N(ee);
    navbarLinks;
    isVisible = !1;
    isMobile = !0;
    isDesktop = !1;
    isMobileMenuOpen = !1;
    lastScrollTop = 0;
    SCROLL_THRESHOLD = 50;
    onWindowResize() {
      ((this.isMobile = window.innerWidth < 640),
        (this.isDesktop = window.innerWidth >= 1024));
    }
    onWindowScroll() {
      if (!$(this.platformId)) return;
      let e = window.scrollY || document.documentElement.scrollTop;
      (e <= 0
        ? (this.isVisible = !1)
        : e < this.lastScrollTop && e > this.SCROLL_THRESHOLD
          ? (this.isVisible = !0)
          : e > this.lastScrollTop && (this.isVisible = !1),
        (this.lastScrollTop = e),
        this.cdr.detectChanges());
    }
    ngOnInit() {
      ((this.isMobileMenuOpen = !1), this.cdr.detectChanges());
    }
    ngAfterViewInit() {
      $(this.platformId) &&
        ((this.isMobile = window.innerWidth < 640),
        (this.isDesktop = window.innerWidth >= 1024),
        this.cdr.detectChanges());
    }
    toggleMobileMenu() {
      ((this.isMobileMenuOpen = !this.isMobileMenuOpen),
        this.toggleBodyScroll(),
        this.cdr.detectChanges());
    }
    closeMobileMenu() {
      ((this.isMobileMenuOpen = !1),
        this.toggleBodyScroll(),
        this.cdr.detectChanges());
    }
    toggleBodyScroll() {
      $(this.platformId) &&
        (this.isMobileMenuOpen
          ? this.renderer.addClass(document.body, "overflow-hidden")
          : this.renderer.removeClass(document.body, "overflow-hidden"));
    }
    static ɵfac = function (o) {
      return new (o || n)();
    };
    static ɵcmp = D({
      type: n,
      selectors: [["pool-land-sticky-header"]],
      hostVars: 2,
      hostBindings: function (o, r) {
        (o & 1 &&
          S(
            "resize",
            function (P) {
              return r.onWindowResize(P);
            },
            !1,
            K,
          )(
            "scroll",
            function (P) {
              return r.onWindowScroll(P);
            },
            !1,
            K,
          ),
          o & 2 && A("visible", r.isVisible));
      },
      inputs: { navbarLinks: "navbarLinks" },
      ngContentSelectors: De,
      decls: 10,
      vars: 3,
      consts: () => {
        let e;
        e = "Own shares in Asia's next fintech unicorn POOOL";
        let o;
        o = "Sign in";
        let r;
        r =
          " Thousands of investors owning shares in Bali's growing market. POOOL lets you invest in premium properties from " +
          "\uFFFD#13\uFFFD" +
          "$500" +
          "\uFFFD/#13\uFFFD" +
          ", using blockchain and notarized certificates for deals that are secure, transparent, and exclusive. ";
        let u;
        u = " Still have questions? ";
        let P;
        return (
          (P = " Contact us on telegram "),
          [
            e,
            r,
            u,
            P,
            [
              1,
              "flex",
              "w-full",
              "items-center",
              "justify-center",
              "bg-[#001DCA]",
              "py-1",
              "text-xs",
              "tracking-wide",
              "text-white",
            ],
            [
              1,
              "relative",
              "grid",
              "grid-cols-[1fr_max-content_1fr]",
              "content-center",
              "items-center",
              "justify-items-center",
              "gap-4",
              "bg-white",
              "px-6",
              "py-3",
              "lg:px-[48px]",
              "xl:px-[72px]",
            ],
            [
              "routerLink",
              "/",
              "queryParamsHandling",
              "preserve",
              "src",
              "/svg/logo-blue.svg",
              "alt",
              "Logo",
              1,
              "h-9",
              "cursor-pointer",
              "justify-self-start",
            ],
            [1, "order-3", "ml-auto", "flex", "gap-8", "sm:ml-0", "lg:order-2"],
            [
              "aria-label",
              "Toggle menu",
              1,
              "fixed",
              "right-6",
              "top-[31px]",
              "z-[1001]",
              "flex",
              "h-10",
              "w-10",
              "flex-col",
              "items-center",
              "justify-center",
              "gap-[6px]",
              "p-2",
            ],
            [
              1,
              "absolute",
              "right-20",
              "top-1/2",
              "-translate-y-1/2",
              "sm:right-16",
              "lg:right-[48px]",
              "xl:right-[72px]",
            ],
            [
              1,
              "fixed",
              "inset-0",
              "z-[1000]",
              "overflow-auto",
              "bg-[#2B32F9]",
              "text-white",
              2,
              "height",
              "100vh",
              "width",
              "100vw",
            ],
            [
              "queryParamsHandling",
              "preserve",
              1,
              "hover:border-primary-blue",
              "border-b-2",
              "border-transparent",
              "text-sm",
              "font-medium",
              "tracking-wide",
              "transition-all",
              "duration-300",
              "ease-in-out",
              3,
              "routerLink",
              "fragment",
            ],
            [
              "aria-label",
              "Toggle menu",
              1,
              "fixed",
              "right-6",
              "top-[31px]",
              "z-[1001]",
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
              "h-[2px]",
              "w-6",
              "transition-all",
              "duration-300",
              "ease-in-out",
              3,
              "ngClass",
            ],
            [1, "flex", "min-h-screen", "flex-col", "justify-between", "pt-16"],
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
              "text-white",
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
            ["text", o, "variant", "primary-full-width", 1, "w-full"],
            ["variant", "secondary"],
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
              "text-white",
              "hover:underline",
              3,
              "click",
              "routerLink",
              "fragment",
            ],
          ]
        );
      },
      template: function (o, r) {
        (o & 1 &&
          (ie(Ne),
          t(0, "div", 4),
          ne(1),
          F(2, 0),
          te(),
          i(),
          t(3, "header", 5),
          l(4, "img", 6),
          t(5, "div", 7),
          m(6, Le, 2, 0)(7, Me, 4, 15, "button", 8),
          i(),
          m(8, Ie, 2, 0, "div", 9),
          i(),
          m(9, ye, 18, 0, "div", 10)),
          o & 2 &&
            (a(6),
            f(r.isDesktop ? 6 : 7),
            a(2),
            f(r.isMobile ? -1 : 8),
            a(),
            f(r.isMobileMenuOpen && !r.isDesktop ? 9 : -1)));
      },
      dependencies: [L, v, ue, G, ce, k],
      styles: [
        "[_nghost-%COMP%]{display:block;position:fixed;top:0;left:0;right:0;width:100%;z-index:1000;transform:translateY(-100%);transition:transform .3s ease;box-shadow:0 2px 10px #0000001a}.visible[_nghost-%COMP%]{transform:translateY(0)}[_ngcontent-%COMP%]:global(.overflow-hidden){overflow:hidden!important;height:100%!important;position:fixed!important;width:100%!important;touch-action:none!important}",
      ],
      changeDetection: 0,
    });
  }
  return n;
})();
var he = () => ["/terms-and-conditions"],
  xe = () => ["/cookies"],
  Re = () => ["/privacy-policy"],
  be = () => ["/currency-policy"],
  on = (() => {
    class n {
      static ɵfac = function (o) {
        return new (o || n)();
      };
      static ɵcmp = D({
        type: n,
        selectors: [["pool-land-pool-company-info"]],
        decls: 73,
        vars: 8,
        consts: () => {
          let e;
          e = "All rights reserved.";
          let o;
          o =
            " POOOL.app, POOOL.id are websites operated by PT. POOOL INTERNATIONAL GROUP, a foreign-owned company (PMA) based in Indonesia, and a subsidiary of the POOOL holding entity headquartered in Hong Kong. ";
          let r;
          r =
            " By accessing this website and any of its sub-pages, you agree to be bound by our Terms of Service, Privacy Policy, and Risk Disclosure. ";
          let u;
          u = "Website language: ";
          let P;
          P = "A Word Of Caution";
          let p;
          ((p =
            " POOOL does not offer financial, legal, tax, or investment advice. The information provided on this platform is for general informational purposes only and does not constitute an offer to sell, or a solicitation of an offer to buy, any securities or real estate interests. Nothing on this site should be construed as a recommendation or endorsement of any investment opportunity. " +
            "[\uFFFD#23\uFFFD\uFFFD/#23\uFFFD|\uFFFD#24\uFFFD\uFFFD/#24\uFFFD|\uFFFD#25\uFFFD\uFFFD/#25\uFFFD|\uFFFD#26\uFFFD\uFFFD/#26\uFFFD]" +
            "" +
            "[\uFFFD#23\uFFFD\uFFFD/#23\uFFFD|\uFFFD#24\uFFFD\uFFFD/#24\uFFFD|\uFFFD#25\uFFFD\uFFFD/#25\uFFFD|\uFFFD#26\uFFFD\uFFFD/#26\uFFFD]" +
            " All transactions on the platform will be settled in Indonesian Rupiah (IDR), regardless of the original payment currency. Conversion rates are determined at the time of transaction. Prices displayed in USD are provided solely for international investor reference. As required by Indonesian Law (PBI No. 17/2015 and Currency Law No. 7/2011), all transactions conducted within the territory. " +
            "[\uFFFD#23\uFFFD\uFFFD/#23\uFFFD|\uFFFD#24\uFFFD\uFFFD/#24\uFFFD|\uFFFD#25\uFFFD\uFFFD/#25\uFFFD|\uFFFD#26\uFFFD\uFFFD/#26\uFFFD]" +
            "" +
            "[\uFFFD#23\uFFFD\uFFFD/#23\uFFFD|\uFFFD#24\uFFFD\uFFFD/#24\uFFFD|\uFFFD#25\uFFFD\uFFFD/#25\uFFFD|\uFFFD#26\uFFFD\uFFFD/#26\uFFFD]" +
            " Investing in real estate, startups, or any private placement opportunities involves significant risks. Investments are not insured by any government agency and may lose value. POOOL does not guarantee any return on investment. It is the sole responsibility of the investor to conduct their own due diligence and ensure they can bear potential losses. "),
            (p = w(p)));
          let H;
          H = " Info, Documents and Support ";
          let x;
          ((x =
            " For official documents, investment policies, and support, please refer to our Legal Center. Directorate General of Consumer Protection and Orderly Commerce Ministry of Trade of the Republic of Indonesia " +
            "[\uFFFD#35\uFFFD\uFFFD/#35\uFFFD|\uFFFD#36\uFFFD\uFFFD/#36\uFFFD|\uFFFD#37\uFFFD\uFFFD/#37\uFFFD|\uFFFD#39\uFFFD\uFFFD/#39\uFFFD|\uFFFD#40\uFFFD\uFFFD/#40\uFFFD|\uFFFD#41\uFFFD\uFFFD/#41\uFFFD]" +
            "" +
            "[\uFFFD#35\uFFFD\uFFFD/#35\uFFFD|\uFFFD#36\uFFFD\uFFFD/#36\uFFFD|\uFFFD#37\uFFFD\uFFFD/#37\uFFFD|\uFFFD#39\uFFFD\uFFFD/#39\uFFFD|\uFFFD#40\uFFFD\uFFFD/#40\uFFFD|\uFFFD#41\uFFFD\uFFFD/#41\uFFFD]" +
            " For assistance, contact our team via WhatsApp:" +
            "[\uFFFD#35\uFFFD\uFFFD/#35\uFFFD|\uFFFD#36\uFFFD\uFFFD/#36\uFFFD|\uFFFD#37\uFFFD\uFFFD/#37\uFFFD|\uFFFD#39\uFFFD\uFFFD/#39\uFFFD|\uFFFD#40\uFFFD\uFFFD/#40\uFFFD|\uFFFD#41\uFFFD\uFFFD/#41\uFFFD]" +
            "" +
            "\uFFFD#38\uFFFD" +
            "+62 813-2581-7676" +
            "\uFFFD/#38\uFFFD" +
            "" +
            "[\uFFFD#35\uFFFD\uFFFD/#35\uFFFD|\uFFFD#36\uFFFD\uFFFD/#36\uFFFD|\uFFFD#37\uFFFD\uFFFD/#37\uFFFD|\uFFFD#39\uFFFD\uFFFD/#39\uFFFD|\uFFFD#40\uFFFD\uFFFD/#40\uFFFD|\uFFFD#41\uFFFD\uFFFD/#41\uFFFD]" +
            "" +
            "[\uFFFD#35\uFFFD\uFFFD/#35\uFFFD|\uFFFD#36\uFFFD\uFFFD/#36\uFFFD|\uFFFD#37\uFFFD\uFFFD/#37\uFFFD|\uFFFD#39\uFFFD\uFFFD/#39\uFFFD|\uFFFD#40\uFFFD\uFFFD/#40\uFFFD|\uFFFD#41\uFFFD\uFFFD/#41\uFFFD]" +
            " Our virtual office address:" +
            "[\uFFFD#35\uFFFD\uFFFD/#35\uFFFD|\uFFFD#36\uFFFD\uFFFD/#36\uFFFD|\uFFFD#37\uFFFD\uFFFD/#37\uFFFD|\uFFFD#39\uFFFD\uFFFD/#39\uFFFD|\uFFFD#40\uFFFD\uFFFD/#40\uFFFD|\uFFFD#41\uFFFD\uFFFD/#41\uFFFD]" +
            "" +
            "\uFFFD#42\uFFFD" +
            " Jl. Raya Kerobokan No.98, Kerobokan Kelod, Kec. Kuta Utara, Kabupaten Badung, Bali 80361, Indonesien " +
            "\uFFFD/#42\uFFFD" +
            ""),
            (x = w(x)));
          let Y;
          Y = "Why Us";
          let X;
          X = "Platform";
          let z;
          z = "How it works";
          let V;
          V = "Market";
          let j;
          j = " Terms & Conditions ";
          let q;
          q = " Legal ";
          let W;
          W = " Privacy Policy ";
          let U;
          U = " Currency Policy ";
          let J;
          return (
            (J = " Licensed: "),
            [
              e,
              o,
              r,
              u,
              P,
              p,
              H,
              x,
              Y,
              X,
              z,
              V,
              j,
              q,
              W,
              U,
              J,
              [
                1,
                "max-block-width",
                "grid",
                "w-full",
                "items-start",
                "justify-center",
                "gap-4",
                "px-4",
                "py-16",
                "text-sm",
                "text-white",
                "lg:px-16",
                "xl:grid-cols-[1fr_max-content_1fr_max-content_1fr]",
                "xl:gap-8",
                "xl:px-28",
                "xl:py-16",
              ],
              [1, "flex", "flex-col", "gap-4"],
              [
                "src",
                "/svg/3-circles-logo.svg",
                "alt",
                "Logo",
                1,
                "max-w-[80px]",
              ],
              [
                1,
                "text-accent-green",
                "text-2xl",
                "uppercase",
                "tracking-tighter",
                "lg:text-3xl",
              ],
              [1, "text-accent-green"],
              [1, "flex", "items-center", "gap-2"],
              ["variant", "secondary"],
              [1, "relative", "w-full", "xl:h-full"],
              [
                1,
                "border-accent-green",
                "w-full",
                "border-l-[1px]",
                "border-solid",
                "xl:h-full",
                "xl:w-[0px]",
              ],
              [1, "flex", "flex-col"],
              [1, "text-xl", "font-semibold", "lg:text-2xl"],
              [
                "target",
                "_blank",
                "href",
                "https://wa.me/6281325817676",
                1,
                "text-secondary-green",
                "font-semibold",
              ],
              [1, "text-secondary-green", "font-semibold"],
              [
                1,
                "flex",
                "flex-wrap",
                "items-start",
                "justify-center",
                "gap-4",
                "text-center",
                "font-bold",
              ],
              [
                "routerLink",
                "/",
                "fragment",
                "why-us",
                "queryParamsHandling",
                "preserve",
              ],
              [
                "routerLink",
                "/",
                "fragment",
                "solution",
                "queryParamsHandling",
                "preserve",
              ],
              [
                "routerLink",
                "/",
                "fragment",
                "how-it-works",
                "queryParamsHandling",
                "preserve",
              ],
              [
                "routerLink",
                "/",
                "fragment",
                "market",
                "queryParamsHandling",
                "preserve",
              ],
              [
                "routerLink",
                "/",
                "fragment",
                "faq",
                "queryParamsHandling",
                "preserve",
              ],
              ["queryParamsHandling", "preserve", 3, "routerLink"],
              [
                "href",
                "https://drive.google.com/drive/folders/1uP5cScXp-6NarP9a7fjHCRjMzujpOuBG",
                "target",
                "_blank",
              ],
              [
                1,
                "mt-8",
                "flex",
                "flex-col",
                "items-start",
                "gap-5",
                "xl:mt-0",
              ],
              [1, "text-xl", "font-semibold", "text-[#FAFAFA]", "lg:text-2xl"],
              [
                1,
                "flex",
                "h-[75px]",
                "w-[93px]",
                "flex-col",
                "items-center",
                "justify-center",
                "rounded",
                "bg-[#F3F8FB]",
                "p-2",
              ],
              [
                "src",
                "/png/license.webp",
                "alt",
                "Logo Kementerian Komunikasi dan Digital Republik Indonesia 2024",
                1,
                "h-[59px]",
                "w-[69px]",
                "object-contain",
              ],
            ]
          );
        },
        template: function (o, r) {
          (o & 1 &&
            (t(0, "div", 17)(1, "div", 18),
            l(2, "img", 19),
            t(3, "h3", 20),
            d(4, " \xA9 PT. POOOL INTERNATIONAL GROUP "),
            i(),
            t(5, "span", 21),
            F(6, 0),
            i(),
            t(7, "p"),
            F(8, 1),
            i(),
            t(9, "p"),
            F(10, 2),
            i(),
            t(11, "div", 22)(12, "span"),
            F(13, 3),
            i(),
            l(14, "pool-land-language-selector", 23),
            i()(),
            t(15, "div", 24),
            l(16, "hr", 25),
            i(),
            t(17, "div", 26)(18, "h3", 27),
            F(19, 4),
            i(),
            l(20, "br"),
            t(21, "p"),
            T(22, 5),
            l(23, "br")(24, "br")(25, "br")(26, "br"),
            y(),
            i()(),
            t(27, "div", 24),
            l(28, "hr", 25),
            i(),
            t(29, "div", 26)(30, "h3", 27),
            F(31, 6),
            i(),
            l(32, "br"),
            t(33, "p"),
            T(34, 7),
            l(35, "br")(36, "br")(37, "br")(38, "a", 28)(39, "br")(40, "br")(
              41,
              "br",
            )(42, "span", 29),
            y(),
            i(),
            l(43, "br")(44, "br"),
            t(45, "div", 30)(46, "a", 31),
            F(47, 8),
            i(),
            t(48, "a", 32),
            F(49, 9),
            i(),
            t(50, "a", 33),
            F(51, 10),
            i(),
            t(52, "a", 34),
            F(53, 11),
            i(),
            t(54, "a", 35),
            d(55, "FAQ"),
            i(),
            t(56, "button", 36),
            F(57, 12),
            i(),
            t(58, "a", 37),
            F(59, 13),
            i(),
            t(60, "button", 36),
            d(61, " Cookies "),
            i(),
            t(62, "button", 36),
            F(63, 14),
            i(),
            t(64, "button", 36),
            F(65, 15),
            i()(),
            l(66, "br")(67, "br"),
            t(68, "div", 38)(69, "h3", 39),
            F(70, 16),
            i(),
            t(71, "div", 40),
            l(72, "img", 41),
            i()()()()),
            o & 2 &&
              (a(56),
              c("routerLink", h(4, he)),
              a(4),
              c("routerLink", h(5, xe)),
              a(2),
              c("routerLink", h(6, Re)),
              a(2),
              c("routerLink", h(7, be))));
        },
        dependencies: [L, G, k],
        styles: [
          "[_nghost-%COMP%]{--tw-bg-opacity: 1;background-color:rgb(43 50 249 / var(--tw-bg-opacity, 1))}",
        ],
        changeDetection: 0,
      });
    }
    return n;
  })();
export {
  k as a,
  Ue as b,
  on as c,
}; /**i18n:e2f94bf06bdfc8c8ab493a12299261c375fc525ae09e041ca331cb13279050ab*/
