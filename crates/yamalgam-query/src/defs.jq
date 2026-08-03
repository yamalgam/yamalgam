# yg definitions loaded after jaq-core and jaq-std.

def type: _yaml_type;
def keys: keys_unsorted;
def keys_sorted: keys_unsorted | sort;

def have_decnum: false;
def nan: _nan;
def infinite: _infinite;
def isnan: _is_nan;
def isinfinite: _is_infinite;
def isfinite: _is_finite;
def isnormal: _is_normal;

def totype(p; e): if p then . else fromjson | if p then . else e end end;
def tonumber : totype(isnumber ; error("cannot parse as number" ));
def toboolean: totype(isboolean; error("cannot parse as boolean"));

def transpose: [range([.[] | length] | max) as $i | [.[][$i]]];

def pick(f):
  reduce path_value(f) as [$path, $value] (null; setpath($path; $value));

def trimstr($text): ltrimstr($text) | rtrimstr($text);

def join($separator):
  [
    .[] |
    if . == null then ""
    elif type == "string" or type == "integer" or type == "float" or type == "boolean"
    then tostring
    else error("join values must be scalars")
    end
  ] |
  reduce .[] as $value (null;
    if . == null then $value else . + $separator + $value end
  ) // "";

def match(re; flags):
  matches(re; flags)[] |
  .[0] + {captures: [.[1:][] | . + {name: (.name // null)}]};
def match(re):
  re as $re |
  if ($re | type) == "sequence" then match($re[0]; $re[1]) else match($re; "") end;

def scan(re; flags):
  matches(re; flags + "g")[] |
  if length > 1 then [.[1:][] | .string] else .[0].string end;
def scan(re): scan(re; "");

def in(xs)    : . as $x | xs | has     ($x);
def inside(xs): . as $x | xs | contains($x);
def  index($i): indices($i)[ 0];
def rindex($i): indices($i)[-1];

def @json: tojson;
