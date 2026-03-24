use minijinja::{Environment, context};

fn main() {
    let mut env = Environment::new();
    env.add_template(
        "test",
        "
{% macro p_icon(**kwargs) %}
<p>{{ kwargs.caller() }}</p>
{% endmacro %}

{% macro wrapper(**kwargs) %}
<div>
  {% call p_icon() %}
    {{ kwargs.caller() }}
  {% endcall %}
</div>
{% endmacro %}

{% call wrapper() %}
  hello
{% endcall %}
"
    ).unwrap();
    let tmpl = env.get_template("test").unwrap();
    match tmpl.render(context! {}) {
        Ok(s) => println!("Success: {}", s),
        Err(e) => println!("Error: {}", e),
    }
}
