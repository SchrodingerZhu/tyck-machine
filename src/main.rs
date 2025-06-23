use std::{collections::HashMap, io::Empty, ops::Deref};
use std::fmt::{Debug, Write};
use gc_arena::collect::Trace;
use gc_arena::{Arena, Collect, Gc, GcLock, GcRefLock, GcWeak, Mutation, RefLock, Rootable};
use gc_arena::arena::CollectionPhase;

#[derive(Collect, Clone)]
#[collect(require_static)]
enum TyVarKind {
    Universal,
    Existential,
}

#[derive(Collect)]
#[collect(no_drop)]
enum Ty<'a> {
    Unit,
    Var(usize),
    Forall(String, Gc<'a, Self>),
    Arrow(Gc<'a, Self>, Gc<'a, Self>),
}

impl<'a> Ty<'a> {
    fn trivially_equal(lhs: &Self, rhs: &Self) -> bool {
        match (lhs, rhs) {
            (Ty::Unit, Ty::Unit) => true,
            (Ty::Var(l), Ty::Var(r)) => l == r,
            _ => false,
        }
    }
}

#[derive(Collect)]
#[collect(no_drop)]
enum Expr<'a> {
    Var(usize),
    App(Gc<'a, Self>, Gc<'a, Self>),
    Lam(String, Gc<'a, Self>),
    Let(String, Gc<'a, Self>, Gc<'a, Self>),
    Unit,
}

#[derive(Collect)]
#[collect(no_drop)]
enum Judgment<'a> {
    Subtype {
        lhs: Gc<'a, Ty<'a>>,
        rhs: Gc<'a, Ty<'a>>,
    },
    Check {
        expr: Gc<'a, Expr<'a>>,
        ty: Gc<'a, Ty<'a>>,
    },
    Infer {
        expr: Gc<'a, Expr<'a>>,
        replace: usize,
        nested: Gc<'a, Self>,
    },
    AppInfer {
        func: Gc<'a, Ty<'a>>,
        expr: Gc<'a, Expr<'a>>,
        replace: usize,
        nested: Gc<'a, Self>,
    },
}

#[derive(Collect, Clone)]
#[collect(no_drop)]
enum WorkItem<'a> {
    TyVarDecl(usize, TyVarKind),
    VarDecl(usize, Gc<'a, Ty<'a>>),
    Judgment(Gc<'a, Judgment<'a>>),
    Garbage,
}

#[derive(Collect, Clone, Copy)]
#[collect(require_static)]
enum Direction {
    Left,
    Right,
}

impl std::ops::Not for Direction {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

#[derive(Collect)]
#[collect(no_drop)]
struct ListNode<'a, T> {
    prev: NodePtr<'a, T>,
    next: NodePtr<'a, T>,
    item: Option<T>,
}

type NodePtr<'a, T> = GcRefLock<'a, ListNode<'a, T>>;
impl<'a, T: Collect<'a>> ListNode<'a, T> {
    fn new(mc: &Mutation<'a>, item: Option<T>, prev: NodePtr<'a, T>, next: NodePtr<'a, T>) -> NodePtr<'a, T> {
        Gc::new(
            mc,
            RefLock::new(ListNode {
                prev,
                next,
                item,
            }),
        )
    }
    fn empty(mc: &Mutation<'a>) -> NodePtr<'a, T> {
        // It is okay to create invalid Gc as such fake pointers are never traced.
        let fake_gc = unsafe { std::mem::transmute(std::ptr::null_mut::<ListNode<'a, T>>()) };
        let obj = Self::new(mc, None, fake_gc, fake_gc);
        obj.borrow_mut(mc).prev = obj;
        obj.borrow_mut(mc).next = obj;
        obj
    }

    fn insert_before(
        this: NodePtr<'a, T>,
        mc: &Mutation<'a>,
        item: T,
    ) -> NodePtr<'a, T> {
        let prev = this.borrow().prev;
        let new_node = Self::new(mc, Some(item), prev, this);
        prev.borrow_mut(mc).next = new_node;
        this.borrow_mut(mc).prev = new_node;
        new_node
    }

    fn unlink(this : NodePtr<'a, T>, mc: &Mutation<'a>) {
        if this.borrow().item.is_none() {
            unreachable!("unlink called on empty ListNode");
        }
        let prev = this.borrow().prev;
        let next = this.borrow().next;
        prev.borrow_mut(mc).next = next;
        next.borrow_mut(mc).prev = prev;
    }
}

#[derive(Collect)]
#[collect(no_drop)]
struct List<'a, T> {
    sentinel: NodePtr<'a, T>,
}

impl<'a, T: Collect<'a>> List<'a, T> {
    fn new(mc: &Mutation<'a>) -> Self {
        Self {
            sentinel: ListNode::empty(mc),
        }
    }

    fn push_back(&self, mc: &Mutation<'a>, item: T) -> NodePtr<'a, T> {
        ListNode::insert_before(self.sentinel, mc, item)
    }

    fn push_front(&self, mc: &Mutation<'a>, item: T) -> NodePtr<'a, T> {
        let last = self.sentinel.borrow().next;
        ListNode::insert_before(last, mc, item)
    }

    fn pop_back(&self, mc: &Mutation<'a>) -> Option<NodePtr<'a, T>> {
        let last = self.sentinel.borrow().prev;
        if last.borrow().item.is_none() {
            return None;
        }
        ListNode::unlink(last, mc);
        Some(last)
    }

    fn pop_back_cloned(&self, mc: &Mutation<'a>) -> Option<T> where T: Clone {
        let last = self.sentinel.borrow().prev;
        let item = last.borrow().item.clone();
        if item.is_none() {
            return None;
        }
        ListNode::unlink(last, mc);
        item
    }

    fn is_empty(&self) -> bool {
        Gc::ptr_eq(self.sentinel.borrow().next, self.sentinel)
    }
}

impl<'a, T: Collect<'a> + Debug> Debug for List<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char('[')?;
        let mut comma = false;
        let mut cursor = self.sentinel.borrow().next;
        loop {
            {
                let item = &cursor.borrow().item;
                match item {
                    Some(item) => {
                        if comma {
                            f.write_str(", ")?;
                        }
                        write!(f, "{:?}", item)?;
                    }
                    None => {
                        f.write_str("]")?;
                        return Ok(());
                    }
                }
            }
            cursor = cursor.borrow().next;
            comma = true;
        }
    }
}

type ItemPtr<'a> = Gc<'a, WorkItem<'a>>;
#[derive(Collect)]
#[collect(no_drop)]
struct TyVarDecl<'a> {
    position: NodePtr<'a, ItemPtr<'a>>,
    kind: TyVarKind,
}

#[derive(Collect)]
#[collect(no_drop)]
struct VarDecl<'a> {
    position: NodePtr<'a, ItemPtr<'a>>,
    ty: Gc<'a, Ty<'a>>,
}
#[derive(Collect)]
#[collect(no_drop)]
struct TyckMachine<'a> {
    worklist: List<'a, ItemPtr<'a>>,
    ty_vars: HashMap<usize, TyVarDecl<'a>>,
    vars: HashMap<usize, TyVarDecl<'a>>,
    replacement: HashMap<usize, Gc<'a, Ty<'a>>>,
}

impl<'a> TyckMachine<'a> {
    fn process_subtype(
        &mut self,
        mc: &Mutation<'a>,
        lhs: &Ty<'a>,
        rhs: &Ty<'a>,
    ) -> anyhow::Result<()> {
        if Ty::trivially_equal(lhs, rhs) {
            return Ok(());
        }
        match (lhs, rhs) {
            (Ty::Arrow(a1, a2), Ty::Arrow(b1, b2)) => {
                let a2_sub_b2 = Gc::new(mc, Judgment::Subtype {
                    lhs: *a2,
                    rhs: *b2,
                });
                let b1_sub_a1 = Gc::new(mc,Judgment::Subtype {
                    lhs: *b1,
                    rhs: *a1,
                });
                self.worklist.push_back(mc, Gc::new(mc, WorkItem::Judgment(a2_sub_b2)));
                self.worklist.push_back(mc, Gc::new(mc, WorkItem::Judgment(b1_sub_a1)));
            }
            _ => todo!("Handle other cases"),
        }
        todo!("")
    }
    fn process_judgment(
        &mut self,
        mutation: &Mutation<'a>,
        judgment: &Judgment<'a>,
    ) -> anyhow::Result<()> {
        match judgment {
            Judgment::Subtype { lhs, rhs } => {
                return self.process_subtype(mutation, lhs.as_ref(), rhs.as_ref());
            }
            _ => todo!()
        }
    }
    fn step(&mut self, mutation: &Mutation<'a>) -> anyhow::Result<bool> {
      match self.worklist.pop_back_cloned(mutation) {
            Some(item) => {
                match item.as_ref() {
                    WorkItem::TyVarDecl(id, _) => {
                        self.ty_vars.remove(id);
                    }
                    WorkItem::VarDecl(id, _) => {
                        self.vars.remove(id);
                    }
                    WorkItem::Judgment(judgment) => {
                        self.process_judgment(mutation, judgment.as_ref())?;
                    }
                    WorkItem::Garbage => {}
                }
                Ok(false)
            }
            None => Ok(true), // No more work items left
        }
    }
}

struct Machine(Arena::<Rootable![TyckMachine<'_>]>);

impl Machine {
    fn new() -> Self {
        let arena = Arena::new(|mc|
            TyckMachine {
                worklist: List::new(mc),
                ty_vars: HashMap::new(),
                vars: HashMap::new(),
                replacement: HashMap::new(),
            }
        );
        Machine(arena)
    }

    pub fn enter<F, T>(&mut self, f: F) -> T
    where
        F: for<'gc> FnOnce(&Mutation<'gc>, &mut TyckMachine<'gc>) -> T,
    {
        const COLLECTOR_GRANULARITY: f64 = 1024.0;

        let r = self.0.mutate_root(move |mc, state| f(mc, state));
        if self.0.metrics().allocation_debt() > COLLECTOR_GRANULARITY {
            if self.0.collection_phase() == CollectionPhase::Sweeping {
                self.0.collect_debt();
            } else {
                if let Some(marked) = self.0.mark_debt() {
                    // Immediately transition to `CollectionPhase::Sweeping`.
                    marked.start_sweeping();
                }
            }
        }
        r
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        loop {
            if self.enter(|mc, state| state.step(mc))? {
                break;
            }
        }
        Ok(())
    }
}

fn main() {
    let mut machine = Machine::new();
    if let Err(e) = machine.run() {
        eprintln!("Error: {}", e);
    }
}
