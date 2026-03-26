use minijinja::{context, Environment};

fn main() {
    let mut env = Environment::new();
    env.add_template(
        "test",
        "
{% macro p_icon(caller=none) %}
<p>{{ caller() }}</p>
{% endmacro %}

{% macro wrapper(caller=none) %}
<div>
  {% call p_icon() %}
    {{ caller() }}
  {% endcall %}
</div>
{% endmacro %}

{% call wrapper() %}
  hello
{% endcall %}
",
    )
    .unwrap();
    let tmpl = env.get_template("test").unwrap();
    match tmpl.render(context! {}) {
        Ok(s) => println!("Success: {}", s),
        Err(e) => println!("Error: {}", e),
    }
}
