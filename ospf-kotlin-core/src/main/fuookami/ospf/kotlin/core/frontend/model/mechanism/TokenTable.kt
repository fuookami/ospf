package fuookami.ospf.kotlin.core.frontend.model.mechanism

import java.util.*
import kotlin.collections.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.Category
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*

class RepeatedSymbolException(
    val repeatedSymbol: Symbol<*>,
    val symbol: Symbol<*>
) : Throwable() {
    override val message get() = "Repeated \"${symbol.name}\", old: $repeatedSymbol, new: $symbol."
}

sealed class TokenTable<C : Category>(
    val tokenList: TokenList,
    private val _symbols: HashMap<String, Symbol<C>> = HashMap()
) {
    val tokens: Collection<Token> get() = tokenList.tokens
    val symbols: Collection<Symbol<C>> get() = _symbols.values
    val solverIndexMap get() = tokenList.solverIndexMap

    fun token(item: Item<*, *>): Token? = tokenList.find(item)
    fun setSolution(solution: List<Flt64>) = tokenList.setResults(solution)
    fun setSolution(solution: Map<Item<*, *>, Flt64>) = tokenList.setResults(solution)
    fun clearSolution() = tokenList.clearResults()

    fun add(item: Item<*, *>): TokenTable<C> {
        tokenList.add(item)
        return this
    }

    fun add(items: Combination<*, *, *>): TokenTable<C> {
        tokenList.add(items)
        return this
    }

    @JvmName("addVarView")
    fun add(items: CombinationView<*, *>): TokenTable<C> {
        tokenList.add(items)
        return this
    }


    fun remove(item: Item<*, *>): TokenTable<C> {
        tokenList.remove(item)
        return this
    }

    @Throws(RepeatedSymbolException::class)
    fun add(symbol: Symbol<C>): TokenTable<C> {
        if (_symbols.containsKey(symbol.name)) {
            throw RepeatedSymbolException(_symbols[symbol.name]!!, symbol)
        } else {
            _symbols[symbol.name] = symbol
            if (symbol is fuookami.ospf.kotlin.core.frontend.expression.symbol.Function) {
                symbol.register(this)
            }
        }
        return this
    }

    @Throws(RepeatedSymbolException::class)
    fun add(symbols: SymbolCombination<C, *>): TokenTable<C> {
        symbols.iterator().forEach { add(it!!) }
        return this
    }

    @JvmName("addSymbolView")
    @Throws(RepeatedSymbolException::class)
    fun add(symbols: SymbolView<C>): TokenTable<C> {
        symbols.iterator().forEach { add(it!!) }
        return this
    }
}

class AutoAddTokenTable<C : Category>() : TokenTable<C>(AutoAddTokenTokenList())

class ManualAddTokenTable<C : Category>() : TokenTable<C>(ManualAddTokenTokenList())
