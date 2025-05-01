package fuookami.ospf.java.example.core_demo;

import java.lang.Integer;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Objects;
import java.util.concurrent.ExecutionException;
import java.util.stream.Collectors;

import fuookami.ospf.kotlin.utils.math.*;
import fuookami.ospf.kotlin.utils.error.Error;
import fuookami.ospf.kotlin.utils.functional.*;
import fuookami.ospf.kotlin.utils.multi_array.*;
import fuookami.ospf.kotlin.core.frontend.variable.*;
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*;
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*;
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*;
import fuookami.ospf.kotlin.core.frontend.inequality.*;
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*;
import fuookami.ospf.kotlin.core.backend.solver.*;
import fuookami.ospf.kotlin.core.backend.solver.output.*;
import fuookami.ospf.kotlin.core.backend.plugins.scip.*;

public class Demo1 {
    class Company {
        public Integer index;
        public Double capital;
        public Double liability;
        public Double profit;

        public Company(Integer index, Double capital, Double liability, Double profit) {
            this.index = index;
            this.capital = capital;
            this.liability = liability;
            this.profit = profit;
        }
    }

    private final List<Company> companies = Arrays.asList(
            new Company(0, 3.48, 1.28, 5400.0),
            new Company(1, 5.62, 2.53, 2300.0),
            new Company(2, 7.33, 1.02, 4600.0),
            new Company(3, 6.27, 3.55, 3300.0),
            new Company(4, 2.14, 0.53, 900.0)
    );
    private final Double minCapital = 10.0;
    private final Double maxLiability = 5.0;

    private BinVariable1 x;
    private LinearExpressionSymbol capital;
    private LinearExpressionSymbol liability;
    private LinearExpressionSymbol profit;

    private final LinearMetaModel metaModel = new LinearMetaModel();

    public void invoke() {
        metaModel.setName("demo1");
        try {
            initVariable();
            initSymbol();
            initObject();
            initConstraint();
            List<Double> solution = solve();
            if (solution!= null) {
                analyzeSolution(solution);
            }
        } catch (Exception e) {
            e.printStackTrace();
        }
    }

    private void initVariable() {
        x = new BinVariable1("x", new Shape1(this.companies.size()));
        for (Company c : companies) {
            x.get(c.index).setName(String.format("%s_%d", x.getName(), c.index));
        }
        x.forEach(xi -> metaModel.add(xi));
    }

    private void initSymbol() {
        capital = LinearExpressionSymbol.Companion.invoke(
                LinearPolynomialKt.sumLinearMonomials(
                    companies.stream().map(c -> LinearMonomialKt.times(c.capital, x.get(c.index))).collect(Collectors.toList())
                ).toMutable(),
                "capital",
                null
        );
        metaModel.add(capital);

        liability = LinearExpressionSymbol.Companion.invoke(
                LinearPolynomialKt.sumLinearMonomials(
                    companies.stream().map(c -> LinearMonomialKt.times(c.liability, x.get(c.index))).collect(Collectors.toList())
                ).toMutable(),
                "liability",
                null
        );
        metaModel.add(liability);

        profit = LinearExpressionSymbol.Companion.invoke(
                LinearPolynomialKt.sumLinearMonomials(
                    companies.stream().map(c -> LinearMonomialKt.times(c.profit, x.get(c.index))).collect(Collectors.toList())
                ).toMutable(),
                "profit",
                null
        );
        metaModel.add(profit);
    }

    private void initObject() {
        metaModel.maximize(
                profit,
                "profit",
                null
        );
    }

    private void initConstraint() {
        metaModel.addConstraint(
                LinearInequalityKt.geq(capital, minCapital),
                "capital_constraint",
                null,
                false
        );
        metaModel.addConstraint(
                LinearInequalityKt.leq(liability, maxLiability),
                "liability_constraint",
                null,
                false
        );
    }

    private List<Double> solve() throws ExecutionException, InterruptedException {
        LinearSolver solver = new ScipLinearSolver();
        Result<SolverOutput, Error> result = solver.solveAsync(metaModel, null).get();
        if (result != null && result.getOk()) {
            List<Flt64> solution = Objects.requireNonNull(result.getValue()).getSolution();
            metaModel.setSolution(solution);
            return solution.stream().map(Flt64::toDouble).collect(Collectors.toList());
        } else {
            return null;
        }
    }

    private void analyzeSolution(List<Double> solution) {
        ArrayList<Company> result = new ArrayList<>();
        for (Token token : metaModel.getTokens().getTokens()) {
            // if (token.belongsTo(x) && Math.abs(solution.get(token.getSolverIndex()) - 1.0) < 1e-5) {
            if (token.belongsTo(x) && Math.abs(Objects.requireNonNull(token.getDoubleResult()) - 1.0) < 1e-5) {
                int[] vector = token.getVariable().getVectorView();
                result.add(companies.get(vector[0]));
            }
        }
    }
}
